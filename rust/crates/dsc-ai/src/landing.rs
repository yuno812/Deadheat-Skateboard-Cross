//! 着地合わせ：空中で、着地点の地面の傾きに板を合わせる（仕様書 E3）。
//! 回し方は復帰処理と同じく「gain × 目標との角度の差 − damping × 角速度」の符号で決める。
//! 目標は、地形から予測した着地点の地面の傾き。予測できなければ真上（0°）。
//! （2つの車輪の真下の床を結んだ傾きに合わせる方法も試したが、R2500 では成功率が下がった）

use serde::{Deserialize, Serialize};

use crate::BodyObs;
use crate::terrain::Terrain;

/// E3 で探索する数値
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LandingParams {
    /// 効きの強さ：目標との角度の差 1° あたりの目標角速度（度/秒）
    pub gain: f32,
    /// 行き過ぎ防止の強さ：角速度にかける重み
    pub damping: f32,
    /// 何もしない角度の幅（度）
    pub deadband_deg: f32,
}

impl Default for LandingParams {
    /// E3 の前の仮の値（E2 の結果に近いもの）
    fn default() -> Self {
        Self { gain: 8.0, damping: 1.0, deadband_deg: 3.0 }
    }
}

/// 着地を予測する時間の上限（秒）
const HORIZON_S: f32 = 3.0;

/// 着地合わせに使う、キャラの形の値
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wheels {
    /// 体の中心から車輪の下端までの距離（まっすぐ立ったとき）
    pub ride_height: f32,
}

/// 1ステップぶんの回転入力（-1 / 0 / 1）
pub fn landing_rotate(params: &LandingParams, me: &BodyObs, terrain: &Terrain, gravity_y: f32, wheels: Wheels) -> f32 {
    rotate_toward(params, me, landing_target(me, terrain, gravity_y, wheels))
}

/// 目標の角度：予測した着地点の地面の傾き。予測できなければ 0°
pub fn landing_target(me: &BodyObs, terrain: &Terrain, gravity_y: f32, wheels: Wheels) -> f32 {
    terrain
        .predict_landing([me.pos.x, me.pos.y], [me.vel.x, me.vel.y], gravity_y, wheels.ride_height, HORIZON_S)
        .map_or(0.0, |l| l.surface_angle_deg)
}

/// 目標の角度へ回す向き
pub fn rotate_toward(params: &LandingParams, me: &BodyObs, target_deg: f32) -> f32 {
    let error = wrap(target_deg - me.angle_deg);
    if error.abs() < params.deadband_deg && me.ang_vel_deg.abs() < params.gain * params.deadband_deg {
        return 0.0;
    }
    let s = params.gain * error - params.damping * me.ang_vel_deg;
    if s > 0.0 {
        1.0
    } else if s < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// -180..180 に直す
fn wrap(a: f32) -> f32 {
    let d = (a + 180.0).rem_euclid(360.0) - 180.0;
    if d == -180.0 { 180.0 } else { d }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(angle_deg: f32, ang_vel_deg: f32) -> BodyObs {
        BodyObs { angle_deg, ang_vel_deg, ..Default::default() }
    }

    #[test]
    fn turns_the_short_way_to_the_target() {
        let p = LandingParams::default();
        assert_eq!(rotate_toward(&p, &obs(0.0, 0.0), 30.0), 1.0);
        assert_eq!(rotate_toward(&p, &obs(170.0, 0.0), -170.0), 1.0);
        assert_eq!(rotate_toward(&p, &obs(30.0, 0.0), 30.0), 0.0);
    }

    #[test]
    fn brakes_near_the_target() {
        let p = LandingParams::default();
        assert_eq!(rotate_toward(&p, &obs(20.0, 500.0), 30.0), -1.0);
    }
}
