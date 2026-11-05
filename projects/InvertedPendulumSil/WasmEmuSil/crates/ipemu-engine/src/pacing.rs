//! 実時間ペーシング (02-architecture.md「仮想時間とペーシング」)。
//!
//! スケジューラスレッドが「いま何ステップ進めるべきか / どれだけ眠るか」をここに問い合わせる。
//! 壁時計との差 (遅れ) が `maxSlip` を超えたら追いつきを諦めて**時間スリップ**として数える
//! (NetworkedSil と同じ)。`debug_friendly` ではブレーク中に溜まった壁時計を捨て、追いつきを一切行わない。

use ipemu_ipc::SchedulerStats;
use std::time::{Duration, Instant};

/// 1 回の問い合わせで返す最大ステップ数。操作キューの応答性を保つための上限
pub const MAX_BATCH: u64 = 4096;

#[derive(Debug)]
pub struct Pacer {
    base_step_ns: i64,
    speed: f64,
    max_slip_ns: i64,
    step_budget: Duration,
    debug_friendly: bool,
    /// 壁時計の原点と、そのときの仮想時刻。resync のたびに取り直す
    origin_wall: Instant,
    origin_virtual_ns: i64,
    // 統計
    slips: u32,
    slip_ms_total: f64,
    step_overruns: u32,
    step_wall_us_ema: f64,
    ratio_window_start: Instant,
    ratio_window_virtual_ns: i64,
    realtime_ratio: f64,
}

/// `due()` の答え。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Due {
    /// 今すぐ `steps` 進める
    Steps(u64),
    /// 次のステップまで眠る
    Sleep(Duration),
}

impl Pacer {
    pub fn new(
        base_step_ns: i64,
        speed: f64,
        max_slip_ms: u32,
        step_budget_ms: u32,
        debug_friendly: bool,
    ) -> Self {
        let now = Instant::now();
        Self {
            base_step_ns: base_step_ns.max(1),
            speed: if speed.is_finite() && speed > 0.0 {
                speed
            } else {
                1.0
            },
            max_slip_ns: (max_slip_ms as i64) * 1_000_000,
            step_budget: Duration::from_millis(step_budget_ms.max(1) as u64),
            debug_friendly,
            origin_wall: now,
            origin_virtual_ns: 0,
            slips: 0,
            slip_ms_total: 0.0,
            step_overruns: 0,
            step_wall_us_ema: 0.0,
            ratio_window_start: now,
            ratio_window_virtual_ns: 0,
            realtime_ratio: 0.0,
        }
    }

    pub fn speed(&self) -> f64 {
        self.speed
    }
    pub fn debug_friendly(&self) -> bool {
        self.debug_friendly
    }

    /// 倍率を変える (原点を取り直す)。
    pub fn set_speed(&mut self, speed: f64, t_ns: i64) {
        if speed.is_finite() && speed > 0.0 {
            self.speed = speed;
        }
        self.resync(t_ns);
    }

    /// 壁時計の原点を「いま = 仮想時刻 t_ns」に取り直す。開始 / 再開 / reset / モード変更で呼ぶ。
    pub fn resync(&mut self, t_ns: i64) {
        self.origin_wall = Instant::now();
        self.origin_virtual_ns = t_ns;
        self.ratio_window_start = self.origin_wall;
        self.ratio_window_virtual_ns = t_ns;
    }

    /// realtime モード: 仮想時刻 `t_ns` に対して、いま進めるべきステップ数か、眠る時間を返す。
    pub fn due(&mut self, t_ns: i64) -> Due {
        let now = Instant::now();
        let wall_ns = now.duration_since(self.origin_wall).as_nanos() as f64 * self.speed;
        let target_ns = self.origin_virtual_ns + wall_ns as i64;
        let deficit = target_ns - t_ns;
        if deficit < self.base_step_ns {
            let wait_ns = ((self.base_step_ns - deficit) as f64 / self.speed) as u64;
            return Due::Sleep(Duration::from_nanos(wait_ns.max(50_000)));
        }
        if self.debug_friendly {
            // デバッガで止まっていた間の壁時計は捨てる。追いつきはしない
            if deficit > self.base_step_ns * 2 {
                self.resync(t_ns + self.base_step_ns);
            }
            return Due::Steps(1);
        }
        if deficit > self.max_slip_ns {
            // 追いつけない遅れ: 時間スリップとして数え、原点をずらす
            self.slips += 1;
            self.slip_ms_total += deficit as f64 * 1e-6;
            self.resync(t_ns + self.base_step_ns);
            return Due::Steps(1);
        }
        Due::Steps(((deficit / self.base_step_ns) as u64).clamp(1, MAX_BATCH))
    }

    /// ステップ実行後の統計更新。`wall` は `steps` ステップにかかった壁時計。
    pub fn record(&mut self, steps: u64, wall: Duration, t_ns: i64) {
        if steps == 0 {
            return;
        }
        let per_step_us = wall.as_secs_f64() * 1e6 / steps as f64;
        self.step_wall_us_ema = if self.step_wall_us_ema == 0.0 {
            per_step_us
        } else {
            self.step_wall_us_ema * 0.98 + per_step_us * 0.02
        };
        if wall
            > self
                .step_budget
                .saturating_mul(steps.min(u32::MAX as u64) as u32)
        {
            self.step_overruns += 1;
        }
        let elapsed = self.ratio_window_start.elapsed();
        if elapsed >= Duration::from_millis(500) {
            let advanced = (t_ns - self.ratio_window_virtual_ns) as f64 * 1e-9;
            self.realtime_ratio = advanced / elapsed.as_secs_f64();
            self.ratio_window_start = Instant::now();
            self.ratio_window_virtual_ns = t_ns;
        }
    }

    /// 停止中は倍率 0 にする。
    pub fn mark_idle(&mut self) {
        self.realtime_ratio = 0.0;
    }

    pub fn stats(&self) -> SchedulerStats {
        SchedulerStats {
            realtime_ratio: self.realtime_ratio,
            step_wall_us: self.step_wall_us_ema,
            slip_ms_total: self.slip_ms_total,
            slips: self.slips,
            step_overruns: self.step_overruns,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleeps_when_ahead_and_steps_when_behind() {
        let mut p = Pacer::new(1_000_000, 1.0, 200, 50, false);
        p.resync(0);
        // 直後: 1ms 経っていないので眠る
        assert!(matches!(p.due(0), Due::Sleep(_)));
        // 5ms 相当遅れている状況を原点をずらして作る
        p.origin_wall = Instant::now() - Duration::from_millis(5);
        match p.due(0) {
            Due::Steps(n) => assert!((4..=6).contains(&n), "{n}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn slip_when_too_far_behind() {
        let mut p = Pacer::new(1_000_000, 1.0, 200, 50, false);
        p.origin_wall = Instant::now() - Duration::from_millis(500);
        assert_eq!(p.due(0), Due::Steps(1));
        assert_eq!(p.stats().slips, 1);
        assert!(p.stats().slip_ms_total >= 499.0);
        // スリップ後は原点が取り直され、次は眠るか少数ステップ
        match p.due(1_000_000) {
            Due::Sleep(_) | Due::Steps(1) => {}
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn debug_friendly_never_catches_up() {
        let mut p = Pacer::new(1_000_000, 1.0, 200, 50, true);
        p.origin_wall = Instant::now() - Duration::from_millis(50);
        assert_eq!(p.due(0), Due::Steps(1));
        assert_eq!(p.stats().slips, 0);
    }

    #[test]
    fn speed_scales_target() {
        let mut p = Pacer::new(1_000_000, 4.0, 200, 50, false);
        p.origin_wall = Instant::now() - Duration::from_millis(2);
        match p.due(0) {
            Due::Steps(n) => assert!((7..=10).contains(&n), "{n}"),
            other => panic!("{other:?}"),
        }
    }
}
