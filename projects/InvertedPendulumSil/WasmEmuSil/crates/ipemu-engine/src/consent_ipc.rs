//! IPC 経由の同意 (M2 のシェル ⇄ M5 の同意機構の結合点)。
//!
//! `Session::load` はスケジューラスレッド上で `ConsentProvider::ask` を同期的に呼ぶ。ここでは
//! 未同意の request を `consent.required` 通知として全接続へ流し、`consent.answer` が揃うまで
//! Condvar で待つ。`consent.answer` は制御器を経由せず IPC サーバから直接 [`ConsentBroker::answer`] に
//! 届く (制御器のスレッドはこの間ブロックしているため)。回答が揃わなければ時間切れで未回答のまま返し、
//! `Session::load` は `LaunchError::ConsentRequired` で失敗する (部分起動はしない)。

use crate::consent::ConsentProvider;
use crate::control::Outbound;
use ipemu_core::consent::{ConsentAnswer, ConsentRequest};
use ipemu_ipc::{codes, Notification, NotificationBody, RpcError};
use std::collections::HashMap;
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

pub struct ConsentBroker {
    pending: Mutex<HashMap<String, Option<ConsentAnswer>>>,
    cv: Condvar,
    out: tokio::sync::mpsc::UnboundedSender<Outbound>,
    timeout: Duration,
}

impl ConsentBroker {
    pub fn new(out: tokio::sync::mpsc::UnboundedSender<Outbound>, timeout: Duration) -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
            cv: Condvar::new(),
            out,
            timeout,
        }
    }

    /// `consent.answer` の受け口。問い合わせ中でない requestId は NOT_FOUND。
    pub fn answer(&self, a: ConsentAnswer) -> Result<(), RpcError> {
        let mut p = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        match p.get_mut(&a.request_id) {
            Some(slot) => {
                *slot = Some(a);
                self.cv.notify_all();
                Ok(())
            }
            None => Err(RpcError::new(
                codes::NOT_FOUND,
                format!("同意要求 `{}` は問い合わせ中ではありません", a.request_id),
            )),
        }
    }

    /// 問い合わせ中の request 数 (テスト用)。
    pub fn pending_count(&self) -> usize {
        self.pending.lock().unwrap_or_else(|e| e.into_inner()).len()
    }
}

fn to_ipc(r: &ConsentRequest) -> ipemu_ipc::ConsentRequest {
    ipemu_ipc::ConsentRequest {
        request_id: r.request_id.clone(),
        instances: r.instances.clone(),
        artifact: r.artifact.clone(),
        artifact_sha256: r.artifact_sha256.clone(),
        capability: r.capability.clone(),
        reason: r.reason.clone(),
        manifest_id: r.manifest_id.clone(),
        revision: r.revision,
    }
}

impl ConsentProvider for ConsentBroker {
    fn ask(&self, requests: &[ConsentRequest]) -> anyhow::Result<Vec<ConsentAnswer>> {
        {
            let mut p = self.pending.lock().unwrap_or_else(|e| e.into_inner());
            p.clear();
            for r in requests {
                p.insert(r.request_id.clone(), None);
            }
        }
        for r in requests {
            let n = Notification::new(NotificationBody::ConsentRequired(to_ipc(r)));
            if let Ok(text) = serde_json::to_string(&n) {
                let _ = self.out.send(Outbound::Text(text));
            }
        }
        let deadline = Instant::now() + self.timeout;
        let mut p = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if p.values().all(|v| v.is_some()) {
                break;
            }
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            let (guard, _) = self
                .cv
                .wait_timeout(p, deadline - now)
                .unwrap_or_else(|e| e.into_inner());
            p = guard;
        }
        let answers: Vec<ConsentAnswer> = p.drain().filter_map(|(_, v)| v).collect();
        Ok(answers)
    }
    fn name(&self) -> &'static str {
        "ipc"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn req(id: &str) -> ConsentRequest {
        ConsentRequest {
            request_id: id.into(),
            instances: vec!["x".into()],
            artifact: "a@1.0.0".into(),
            artifact_sha256: "00".repeat(32),
            capability: "clock".into(),
            reason: "r".into(),
            manifest_id: "m".into(),
            revision: 1,
        }
    }

    #[test]
    fn notifies_and_collects_answers() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let b = Arc::new(ConsentBroker::new(tx, Duration::from_secs(5)));
        let b2 = b.clone();
        let asker = std::thread::spawn(move || b2.ask(&[req("k1"), req("k2")]).unwrap());
        // 通知が 2 件出る
        let mut got = 0;
        while got < 2 {
            match rx.try_recv() {
                Ok(Outbound::Text(t)) => {
                    assert!(t.contains("consent.required"));
                    got += 1;
                }
                _ => std::thread::sleep(Duration::from_millis(5)),
            }
        }
        // 未知 id は NOT_FOUND、既知は受理
        assert_eq!(
            b.answer(ConsentAnswer {
                request_id: "zz".into(),
                grant: true,
                remember: false
            })
            .unwrap_err()
            .code,
            codes::NOT_FOUND
        );
        b.answer(ConsentAnswer {
            request_id: "k1".into(),
            grant: true,
            remember: false,
        })
        .unwrap();
        b.answer(ConsentAnswer {
            request_id: "k2".into(),
            grant: false,
            remember: true,
        })
        .unwrap();
        let mut answers = asker.join().unwrap();
        answers.sort_by(|a, b| a.request_id.cmp(&b.request_id));
        assert_eq!(answers.len(), 2);
        assert!(answers[0].grant && !answers[1].grant && answers[1].remember);
        assert_eq!(b.pending_count(), 0);
    }

    #[test]
    fn times_out_with_partial_answers() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let b = Arc::new(ConsentBroker::new(tx, Duration::from_millis(100)));
        let b2 = b.clone();
        let asker = std::thread::spawn(move || b2.ask(&[req("k1"), req("k2")]).unwrap());
        std::thread::sleep(Duration::from_millis(20));
        b.answer(ConsentAnswer {
            request_id: "k1".into(),
            grant: true,
            remember: false,
        })
        .unwrap();
        let answers = asker.join().unwrap();
        assert_eq!(answers.len(), 1);
    }
}
