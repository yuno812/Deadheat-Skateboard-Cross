//! 復帰処理：倒れた状態から回転だけで起き上がる（仕様書 E1・E2）。
//!
//! 回転の入力は向きしか意味を持たない（-1 / 0 / 1）ので、
//! 「gain × 真上との角度の差 − damping × 角速度」の符号で回す向きを決める。
//! damping が大きいほど、真上の手前で逆向きに回してブレーキをかける（行き過ぎ防止）。
//! 真上に近づかなくなったら、逆向きにしばらく回して揺らす（切り返し）。

use serde::{Deserialize, Serialize};

use crate::BodyObs;

/// E2 で探索する数値
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryParams {
    /// 効きの強さ：真上との角度の差 1° あたりの目標角速度（度/秒）
    pub gain: f32,
    /// 行き過ぎ防止の強さ：角速度にかける重み
    pub damping: f32,
    /// 何もしない角度の幅（度）。真上からこれ以内なら回さない
    pub deadband_deg: f32,
    /// 切り返しの条件：これだけのステップ真上に近づかなければ、逆向きに同じステップ数だけ回す
    pub stall_steps: u32,
    /// 時間切れ：1回の試行がこれを超えたら、いったん離して落ち着かせてからやり直す
    pub timeout_steps: u32,
}

impl Default for RecoveryParams {
    /// E2 の前の仮の値
    fn default() -> Self {
        Self { gain: 5.0, damping: 1.0, deadband_deg: 10.0, stall_steps: 25, timeout_steps: 200 }
    }
}

/// 時間切れの後、手を離して落ち着かせるステップ数
const REST_STEPS: u32 = 25;
/// 「真上に近づいた」とみなす改善の幅（度）
const PROGRESS_DEG: f32 = 1.0;

#[derive(Debug, Clone)]
pub struct Recovery {
    params: RecoveryParams,
    /// この試行で一番真上に近づいた角度の差
    best_error: f32,
    since_best: u32,
    elapsed: u32,
    /// 切り返し・休みのあいだ出し続ける入力と、残りステップ数
    forced: f32,
    forced_left: u32,
    last: f32,
}

impl Recovery {
    pub fn new(params: RecoveryParams) -> Self {
        Self { params, best_error: f32::INFINITY, since_best: 0, elapsed: 0, forced: 0.0, forced_left: 0, last: 0.0 }
    }

    pub fn params(&self) -> &RecoveryParams {
        &self.params
    }

    /// 起き上がった、または別の行動に移ったときに呼ぶ
    pub fn reset(&mut self) {
        *self = Self::new(self.params);
    }

    /// 1ステップぶんの回転入力（-1 / 0 / 1）。左回転（反時計回り）が正
    pub fn update(&mut self, me: &BodyObs) -> f32 {
        let p = self.params;
        // 真上（角度 0）に向かう差。反時計回りに回すと角度が増えるので、差の符号が回す向き
        let error = -me.angle_deg;
        self.elapsed += 1;
        if error.abs() < self.best_error - PROGRESS_DEG {
            self.best_error = error.abs();
            self.since_best = 0;
        } else {
            self.since_best += 1;
        }

        let out = if self.forced_left > 0 {
            self.forced_left -= 1;
            self.forced
        } else if self.elapsed >= p.timeout_steps {
            // 時間切れ：いったん離して落ち着かせ、やり直す
            self.start_forced(0.0, REST_STEPS);
            self.elapsed = 0;
            self.best_error = f32::INFINITY;
            0.0
        } else if self.since_best >= p.stall_steps && self.last != 0.0 {
            // 切り返し：今までと逆向きにしばらく回して揺らす
            self.start_forced(-self.last, p.stall_steps);
            self.since_best = 0;
            self.best_error = error.abs();
            -self.last
        } else if error.abs() < p.deadband_deg {
            0.0
        } else {
            let s = p.gain * error - p.damping * me.ang_vel_deg;
            if s > 0.0 {
                1.0
            } else if s < 0.0 {
                -1.0
            } else {
                0.0
            }
        };
        self.last = out;
        out
    }

    fn start_forced(&mut self, value: f32, steps: u32) {
        self.forced = value;
        self.forced_left = steps.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(angle_deg: f32, ang_vel_deg: f32) -> BodyObs {
        BodyObs { angle_deg, ang_vel_deg, ..Default::default() }
    }

    #[test]
    fn turns_toward_upright() {
        let mut r = Recovery::new(RecoveryParams::default());
        assert_eq!(r.update(&obs(-120.0, 0.0)), 1.0);
        let mut r = Recovery::new(RecoveryParams::default());
        assert_eq!(r.update(&obs(120.0, 0.0)), -1.0);
    }

    #[test]
    fn brakes_before_overshooting() {
        // 真上の手前 20° で、真上に向かって速く回っているときは逆向きに回す
        let mut r = Recovery::new(RecoveryParams { gain: 5.0, damping: 1.0, ..Default::default() });
        assert_eq!(r.update(&obs(-20.0, 600.0)), -1.0);
    }

    #[test]
    fn does_nothing_near_upright() {
        let mut r = Recovery::new(RecoveryParams::default());
        assert_eq!(r.update(&obs(5.0, 0.0)), 0.0);
    }

    #[test]
    fn reverses_when_stalled() {
        let params = RecoveryParams { stall_steps: 3, timeout_steps: 1000, ..Default::default() };
        let mut r = Recovery::new(params);
        // 角度が変わらない（引っかかっている）
        let outs: Vec<f32> = (0..8).map(|_| r.update(&obs(-120.0, 0.0))).collect();
        // 1回目で最良が決まり、3ステップ近づかなかった4回目から3ステップ逆向き
        assert_eq!(&outs[..3], &[1.0, 1.0, 1.0], "{outs:?}");
        assert!(outs[3..6].iter().all(|&o| o == -1.0), "{outs:?}");
    }

    #[test]
    fn rests_after_timeout() {
        let params = RecoveryParams { stall_steps: 1000, timeout_steps: 5, ..Default::default() };
        let mut r = Recovery::new(params);
        let outs: Vec<f32> = (0..10).map(|i| r.update(&obs(-120.0 + i as f32 * 2.0, 0.0))).collect();
        assert_eq!(outs[4], 0.0);
        assert!(outs[5..10].iter().all(|&o| o == 0.0));
    }
}
