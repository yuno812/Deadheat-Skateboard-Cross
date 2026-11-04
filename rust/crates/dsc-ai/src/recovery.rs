//! 復帰処理：倒れた状態から回転だけで起き上がる（仕様書 E1・E2）。
//!
//! 回転の入力は向きしか意味を持たない（-1 / 0 / 1）ので、
//! 「gain × 真上との角度の差 − damping × 角速度」の符号で回す向きを決める。
//! damping が大きいほど、真上の手前で逆向きに回してブレーキをかける（行き過ぎ防止）。
//! 真上に近づかなくなったら、ブランコをこぐように揺らす（切り返し）：まず逆向きに押し、
//! 動き出したら動いている向きに押す。止まったまま押し続けて動かなければ向きを変える。
//! 重くて回転力の弱いキャラ（cubu）が逆さまの「こぶ」を越えるのに必要。

use serde::{Deserialize, Serialize};

use crate::BodyObs;

/// E2 で探索する数値
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RecoveryParams {
    /// 効きの強さ：真上との角度の差 1° あたりの目標角速度（度/秒）
    pub gain: f32,
    /// 行き過ぎ防止の強さ：角速度にかける重み
    pub damping: f32,
    /// 何もしない角度の幅（度）。真上からこれ以内なら回さない
    pub deadband_deg: f32,
    /// 切り返しの条件：これだけのステップ真上に近づかなければ、揺らし始める
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
/// 揺らしをやめる角度（真上からこれ以内まで来たら、普通の制御に戻る）
const PUMP_EXIT_DEG: f32 = 90.0;
/// 揺らしで「動いている」とみなす角速度（度/秒）。これより速ければ動いている向きに押す
const PUMP_SWING_DEG_PER_S: f32 = 30.0;

#[derive(Debug, Clone)]
pub struct Recovery {
    params: RecoveryParams,
    /// この試行で一番真上に近づいた角度の差
    best_error: f32,
    since_best: u32,
    elapsed: u32,
    /// 揺らしている最中
    pumping: bool,
    /// 揺らしで押している向きと、動かないまま押し続けたステップ数
    pump_dir: f32,
    pump_hold: u32,
    /// 時間切れの後の休みの残りステップ数
    rest_left: u32,
    last: f32,
}

impl Recovery {
    pub fn new(params: RecoveryParams) -> Self {
        Self { params, best_error: f32::INFINITY, since_best: 0, elapsed: 0, pumping: false, pump_dir: 0.0, pump_hold: 0, rest_left: 0, last: 0.0 }
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

        if self.pumping && error.abs() < PUMP_EXIT_DEG {
            self.pumping = false;
        }

        let out = if self.rest_left > 0 {
            self.rest_left -= 1;
            0.0
        } else if self.elapsed >= p.timeout_steps {
            // 時間切れ：いったん離して落ち着かせ、やり直す
            self.rest_left = REST_STEPS - 1;
            self.elapsed = 0;
            self.best_error = f32::INFINITY;
            self.pumping = false;
            0.0
        } else if self.pumping || (self.since_best >= p.stall_steps && self.last != 0.0) {
            // 切り返し：動いている向きに押して揺れを大きくする
            if !self.pumping {
                self.pumping = true;
                self.since_best = 0;
                self.best_error = error.abs();
                self.pump_dir = if self.last != 0.0 { -self.last } else { error.signum() };
                self.pump_hold = 0;
            }
            if me.ang_vel_deg.abs() > PUMP_SWING_DEG_PER_S {
                self.pump_dir = me.ang_vel_deg.signum();
                self.pump_hold = 0;
            } else {
                self.pump_hold += 1;
                if self.pump_hold >= p.stall_steps {
                    self.pump_dir = -self.pump_dir;
                    self.pump_hold = 0;
                }
            }
            self.pump_dir
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
    fn pumps_when_stalled() {
        let params = RecoveryParams { stall_steps: 3, timeout_steps: 1000, ..Default::default() };
        let mut r = Recovery::new(params);
        // 角度が変わらない（引っかかっている）
        let outs: Vec<f32> = (0..4).map(|_| r.update(&obs(-120.0, 0.0))).collect();
        // 1回目で最良が決まり、3ステップ近づかなかった4回目から揺らし始める（止まっているので逆向き）
        assert_eq!(outs, [1.0, 1.0, 1.0, -1.0]);
        // 止まったままでも、すぐには向きを変えない
        assert_eq!(r.update(&obs(-120.0, 0.0)), -1.0);
        // 揺らしの最中は、動いている向きに回す
        assert_eq!(r.update(&obs(-130.0, -200.0)), -1.0);
        assert_eq!(r.update(&obs(-140.0, 200.0)), 1.0);
        // 止まったまま stall_steps 押し続けたら向きを変える
        let held: Vec<f32> = (0..3).map(|_| r.update(&obs(-140.0, 0.0))).collect();
        assert_eq!(held, [1.0, 1.0, -1.0]);
        // 真上から 90° 以内まで来たら普通の制御に戻る（ブレーキがかかる）
        assert_eq!(r.update(&obs(-60.0, 800.0)), -1.0);
        assert_eq!(r.update(&obs(-60.0, 0.0)), 1.0);
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
