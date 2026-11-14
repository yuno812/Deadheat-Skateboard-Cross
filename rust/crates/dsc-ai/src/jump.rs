//! ジャンプの軌道予測（仕様書 E4）。ジャンプボタンを離したステップから、体の中心の位置を1ステップずつ予測する。
//!
//! - `naive`：体の中心が放物線を描くとする、素朴な式
//! - `predict`：ゲームの仕組みを入れたもの。照合で分かった次の点を再現する
//!   - 横方向の速度は空中でも毎ステップ ±maxSpeed に抑えられる（MovePlayer.HandleMove）
//!   - 放物線を描くのは重心で、体の中心は重心のまわりを回る（剛体の速度は重心の速度）
//!   - 離したステップでは、移動の減速が先にかかってからジャンプの速度が足される
//!   - 速度を先に、位置を後に更新する（Box2D の積分の順番）

use crate::BodyObs;

/// 予測に使うキャラの値（書き出しの stats と、通常の姿勢の重心）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JumpModel {
    pub jump_force: f32,
    pub max_speed: f32,
    pub deceleration: f32,
    /// 剛体のローカル座標での重心
    pub center_of_mass: [f32; 2],
    pub gravity_y: f32,
    pub dt: f32,
}

/// 素朴な放物線：体の中心が初速 v + up × jumpForce で投げ上げられたとする
pub fn naive(model: &JumpModel, me: &BodyObs, steps: usize) -> Vec<[f32; 2]> {
    let up = up(me.angle_deg);
    let v = [me.vel.x + up[0] * model.jump_force, me.vel.y + up[1] * model.jump_force];
    (1..=steps)
        .map(|k| {
            let t = k as f32 * model.dt;
            [me.pos.x + v[0] * t, me.pos.y + v[1] * t + 0.5 * model.gravity_y * t * t]
        })
        .collect()
}

/// ゲームの仕組みを入れた予測。me は離すステップの直前（前のステップの終わり）の状態で、接地していること
pub fn predict(model: &JumpModel, me: &BodyObs, steps: usize) -> Vec<[f32; 2]> {
    let mut angle = me.angle_deg.to_radians();
    let w = me.ang_vel_deg.to_radians();
    let mut com = add(rotate(model.center_of_mass, angle), [me.pos.x, me.pos.y]);

    // 離すステップ：減速と速度の上限が先、ジャンプの速度が後
    let vx = move_towards(me.vel.x, 0.0, model.deceleration * model.dt).clamp(-model.max_speed, model.max_speed);
    let up = up(me.angle_deg);
    let mut v = [vx + up[0] * model.jump_force, me.vel.y + up[1] * model.jump_force];

    let mut out = Vec::with_capacity(steps);
    for k in 0..steps {
        if k > 0 {
            v[0] = v[0].clamp(-model.max_speed, model.max_speed);
        }
        v[1] += model.gravity_y * model.dt;
        com = [com[0] + v[0] * model.dt, com[1] + v[1] * model.dt];
        angle += w * model.dt;
        out.push(sub(com, rotate(model.center_of_mass, angle)));
    }
    out
}

fn up(angle_deg: f32) -> [f32; 2] {
    let a = angle_deg.to_radians();
    [-a.sin(), a.cos()]
}

fn rotate(p: [f32; 2], a: f32) -> [f32; 2] {
    let (s, c) = a.sin_cos();
    [c * p[0] - s * p[1], s * p[0] + c * p[1]]
}

fn add(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] + b[0], a[1] + b[1]]
}

fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn move_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    if (target - current).abs() <= max_delta { target } else { current + (target - current).signum() * max_delta }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vec2;

    fn model() -> JumpModel {
        JumpModel { jump_force: 9.0, max_speed: 5.0, deceleration: 1.0, center_of_mass: [0.0, 0.0], gravity_y: -9.81, dt: 0.02 }
    }

    #[test]
    fn straight_up_matches_the_unity_trace() {
        // S3：まっすぐ立って静止から跳ぶと、最初のステップの後の上向きの速さは 9 - 9.81 × 0.02
        let me = BodyObs { pos: Vec2 { x: 0.0, y: 0.0 }, ..Default::default() };
        let p = predict(&model(), &me, 2);
        assert!((p[0][1] - (9.0 - 0.1962) * 0.02).abs() < 1e-5, "{p:?}");
    }

    #[test]
    fn horizontal_speed_is_capped_after_the_first_step() {
        // 大きく傾けて跳ぶと横の初速は 5 を超えるが、2ステップ目からは 5 に抑えられる
        let me = BodyObs { angle_deg: -60.0, ..Default::default() };
        let p = predict(&model(), &me, 3);
        let first = p[0][0] / 0.02;
        assert!(first > 5.0, "{first}");
        assert!(((p[2][0] - p[1][0]) / 0.02 - 5.0).abs() < 1e-3, "{p:?}");
        let n = naive(&model(), &me, 3);
        assert!(n[2][0] > p[2][0]);
    }
}
