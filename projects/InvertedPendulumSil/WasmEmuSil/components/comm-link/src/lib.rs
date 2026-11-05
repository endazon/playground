//! comm-link — スカラー信号の通信路エミュレータ。片道遅延 + 一様ジッタ + 再送スパイク + 追い越し禁止。
//!
//! NetworkedSil の DelayLine の部品化。乱数はシード付き PRNG (xorshift64*) なので決定的。

use ip_part_sdk::{num, scalar, PartDesc, PartImpl};
use std::collections::VecDeque;

struct Rng(u64);
impl Rng {
    fn next_f64(&mut self) -> f64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        let r = x.wrapping_mul(0x2545F4914F6CDD1D);
        (r >> 11) as f64 / (1u64 << 53) as f64
    }
}

pub struct Link {
    delay_ns: i64,
    jitter_ns: i64,
    spike_ns: i64,
    spike_prob: f64,
    seed: u64,
    rng: Rng,
    sample_every_ns: i64,
    t_ns: i64,
    next_sample_ns: i64,
    input: f64,
    output: f64,
    queue: VecDeque<(i64, f64)>,
    last_arrival_ns: i64,
    pushed: u64,
    spikes: u64,
    hol_blocked: u64,
}

impl PartImpl for Link {
    fn describe() -> PartDesc {
        PartDesc {
            name: "comm-link".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            inputs: vec![scalar("in", "", "送信側の値 (sampleEveryMs ごとにサンプルして送る)")],
            outputs: vec![
                scalar("out", "", "受信側の値 (到達したものを保持)"),
                scalar("in-flight", "count", "伝送中のサンプル数"),
                scalar("spikes", "count", "注入した再送スパイクの数"),
                scalar("hol-blocked", "count", "追い越し禁止で待たされた数"),
            ],
            params: Vec::new(),
            deterministic: true,
            doc: "遅延 / ジッタ / 再送スパイク。TCP 上の損失は再送による遅延スパイクとして現れるので、捨てずに遅らせる".into(),
        }
    }

    fn init(v: &serde_json::Value) -> Result<Self, String> {
        let delay_ms = num(v, "delayMs", 0.0);
        let jitter_ms = num(v, "jitterMs", 0.0);
        let spike_ms = num(v, "spikeMs", 150.0);
        let spike_percent = num(v, "spikePercent", 0.0);
        let sample_every_ms = num(v, "sampleEveryMs", 1.0);
        if delay_ms < 0.0
            || jitter_ms < 0.0
            || spike_ms < 0.0
            || !(0.0..=100.0).contains(&spike_percent)
            || !(sample_every_ms > 0.0)
        {
            return Err(
                "delayMs/jitterMs/spikeMs >= 0, spikePercent 0..100, sampleEveryMs > 0".into(),
            );
        }
        let seed = num(v, "seed", 1.0) as u64 | 1;
        Ok(Self {
            delay_ns: (delay_ms * 1e6) as i64,
            jitter_ns: (jitter_ms * 1e6) as i64,
            spike_ns: (spike_ms * 1e6) as i64,
            spike_prob: spike_percent / 100.0,
            seed,
            rng: Rng(seed),
            sample_every_ns: (sample_every_ms * 1e6) as i64,
            t_ns: 0,
            next_sample_ns: 0,
            input: 0.0,
            output: 0.0,
            queue: VecDeque::new(),
            last_arrival_ns: 0,
            pushed: 0,
            spikes: 0,
            hol_blocked: 0,
        })
    }

    fn reset(&mut self) {
        self.rng = Rng(self.seed);
        self.t_ns = 0;
        self.next_sample_ns = 0;
        self.input = 0.0;
        self.output = 0.0;
        self.queue.clear();
        self.last_arrival_ns = 0;
        self.pushed = 0;
        self.spikes = 0;
        self.hol_blocked = 0;
    }

    fn set_inputs(&mut self, values: &[f64]) {
        if let Some(v) = values.first() {
            self.input = *v;
        }
    }

    fn step(&mut self, dt_ns: i64) -> Result<(), String> {
        // ステップ開始時刻でサンプルして投入
        if self.t_ns >= self.next_sample_ns {
            let mut delay = self.delay_ns + (self.rng.next_f64() * self.jitter_ns as f64) as i64;
            if self.spike_prob > 0.0 && self.rng.next_f64() < self.spike_prob {
                delay += self.spike_ns;
                self.spikes += 1;
            }
            let mut at = self.t_ns + delay;
            if at < self.last_arrival_ns {
                at = self.last_arrival_ns;
                self.hol_blocked += 1;
            }
            self.last_arrival_ns = at;
            self.queue.push_back((at, self.input));
            self.pushed += 1;
            self.next_sample_ns += self.sample_every_ns;
        }
        self.t_ns += dt_ns;
        // ステップ終了時刻までに到達したものを配送 (最後の値を保持)
        while let Some(&(at, v)) = self.queue.front() {
            if at > self.t_ns {
                break;
            }
            self.output = v;
            self.queue.pop_front();
        }
        Ok(())
    }

    fn get_outputs(&self, out: &mut Vec<f64>) {
        out.extend_from_slice(&[
            self.output,
            self.queue.len() as f64,
            self.spikes as f64,
            self.hol_blocked as f64,
        ]);
    }
}

ip_part_sdk::part_entry!(Link);
