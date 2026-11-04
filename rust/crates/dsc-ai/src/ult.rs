//! ウルトを撃つかの判断。キャラごとに使い方が違う（Assets/Code/Character/*/…ult.cs）。
//!
//! - すぐ撃つ（cubu・maiden・meme）：どこで撃っても損をしないので、溜まったら撃つ
//!   - cubu：自分を追いかけて回復する玉を3個（3秒おき）
//!   - maiden：画面全体に10秒、相手が遅く・跳びにくくなり、自分は速く・高く跳べるエリア
//!   - meme：相手を追いかける遅い弾を2個
//! - 空中ジャンプ（hitsuji）：体の上向きに jumpForce の速度を足す。空中でも出るので、
//!   体がほぼ立っていて下りているときに、(1) 足元に床が無い（場外へ落ちる）なら復帰に、
//!   (2) 今撃つと下りの途中で相手の真上を通るなら踏みつけに使う。
//!   HP が 1 以下だと満タン値が 0.1 になり（HitsujiPassive）、条件を満たすたびに撃てる
//! - それ以外（nagisa・zero・okuda）は撃たない（狙いが要るので未対応／okuda はウルトなし）
//!
//! シミュレーターはウルトを再現しないので、判断だけをここで行い、効果はゲームで確かめる。

use crate::jump::JumpModel;
use crate::terrain::Terrain;
use crate::{BodyObs, Observation};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UltKind {
    /// 撃たない
    #[default]
    None,
    /// 溜まったらすぐ撃つ
    Instant,
    /// 空中ジャンプとして、復帰と踏みつけに使う
    AirJump,
}

/// キャラ名（Prefab のファイル名の小文字）からウルトの使い方を決める
pub fn ult_kind(character: &str) -> UltKind {
    match character {
        "cubu" | "maiden" | "meme" => UltKind::Instant,
        "hitsuji" => UltKind::AirJump,
        _ => UltKind::None,
    }
}

/// 空中ジャンプを使う、体の傾きの上限（度）。これより傾くと上向きの速度が小さく、横や下へ飛んでしまう
const AIR_JUMP_UPRIGHT_DEG: f32 = 50.0;
/// 空中ジャンプの予測の長さ（2秒）
const AIR_JUMP_LOOKAHEAD: usize = 100;
/// 上りながら相手の真下を通るとき、相手の板に胴体が当たるとみなす範囲（横、相手の中心からの高さ）
const RISING_HIT_DX: f32 = 1.0;
const RISING_HIT_DY: (f32, f32) = (-1.6, 0.4);

/// ゲージが満タンか（MovePlayer.HandleSpecialActions と同じ条件）。満タン値が 0 のキャラはウルトなし
pub fn gauge_full(me: &BodyObs) -> bool {
    me.ult_max > 0.0 && me.ult >= me.ult_max - 0.1
}

pub struct UltContext<'a> {
    pub jump: &'a JumpModel,
    pub terrain: &'a Terrain,
    pub ride_height: f32,
    /// 踏みつけとみなす範囲（Tactics の stomp_dx・stomp_dy）
    pub stomp_dx: f32,
    pub stomp_dy: (f32, f32),
}

/// このステップでウルトのボタンを押すか。airborne は「空中にいて、地面に寝てもいない」
pub fn wants_ult(kind: UltKind, obs: &Observation, airborne: bool, cx: &UltContext) -> bool {
    let me = &obs.me;
    if me.hp <= 0.0 || !gauge_full(me) {
        return false;
    }
    match kind {
        UltKind::None => false,
        UltKind::Instant => true,
        UltKind::AirJump => airborne && wants_air_jump(obs, cx),
    }
}

fn wants_air_jump(obs: &Observation, cx: &UltContext) -> bool {
    let (me, opp) = (&obs.me, &obs.opp);
    if me.angle_deg.abs() > AIR_JUMP_UPRIGHT_DEG || me.vel.y >= 0.0 {
        return false;
    }
    let gravity_y = cx.jump.gravity_y;
    // (1) 復帰：このまま落ちると、場外の内側に着地しない
    let landing = cx.terrain.predict_landing([me.pos.x, me.pos.y], [me.vel.x, me.vel.y], gravity_y, cx.ride_height, 3.0);
    let inside = |x: f32| obs.bounds.x == 0.0 || x.abs() < obs.bounds.x.abs();
    if !landing.is_some_and(|l| inside(l.x)) {
        return true;
    }
    // (2) 踏みつけ：撃つと下りの途中で相手の真上を通り、上りで相手の板に当たらない
    let path = air_jump_path(cx.jump, me, AIR_JUMP_LOOKAHEAD);
    let mut prev = [me.pos.x, me.pos.y];
    let mut stomp = false;
    for &p in &path {
        let (dx, dy) = ((p[0] - opp.pos.x).abs(), p[1] - opp.pos.y);
        if p[1] >= prev[1] && dx < RISING_HIT_DX && (RISING_HIT_DY.0..=RISING_HIT_DY.1).contains(&dy) {
            return false;
        }
        if p[1] < prev[1] && dx < cx.stomp_dx && (cx.stomp_dy.0..=cx.stomp_dy.1).contains(&dy) {
            stomp = true;
            break;
        }
        // 床に着いたら、その先は通らない
        if cx.terrain.floor_below(p[0], p[1] + 0.1).is_some_and(|(floor, _)| p[1] - floor < cx.ride_height) {
            break;
        }
        prev = p;
    }
    stomp
}

/// 空中で撃ったときの体の中心の軌道。速度に体の上向き × jumpForce を足し、
/// 横の速度は毎ステップ ±maxSpeed に抑えられる（空中でも MovePlayer.HandleMove が抑える）
pub fn air_jump_path(model: &JumpModel, me: &BodyObs, steps: usize) -> Vec<[f32; 2]> {
    let a = me.angle_deg.to_radians();
    let mut v = [me.vel.x - a.sin() * model.jump_force, me.vel.y + a.cos() * model.jump_force];
    let mut p = [me.pos.x, me.pos.y];
    (0..steps)
        .map(|_| {
            v[0] = v[0].clamp(-model.max_speed, model.max_speed);
            v[1] += model.gravity_y * model.dt;
            p = [p[0] + v[0] * model.dt, p[1] + v[1] * model.dt];
            p
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vec2;

    fn model() -> JumpModel {
        JumpModel { jump_force: 10.0, max_speed: 5.0, deceleration: 1.0, center_of_mass: [0.0, -0.5], gravity_y: -9.81, dt: 0.02 }
    }

    /// 床は x = -6..6、上面 y = -4
    fn terrain() -> Terrain {
        let floor: [[f32; 2]; 4] = [[-6.0, -5.0], [6.0, -5.0], [6.0, -4.0], [-6.0, -4.0]];
        Terrain::from_polygons([&floor[..]])
    }

    fn body(x: f32, y: f32) -> BodyObs {
        BodyObs { pos: Vec2 { x, y }, hp: 3.0, ult: 30.0, ult_max: 30.0, facing: 1.0, ..Default::default() }
    }

    fn obs(me: BodyObs, opp: BodyObs) -> Observation {
        Observation { me, opp, gravity_up: Vec2 { x: 0.0, y: 1.0 }, bounds: Vec2 { x: 9.0, y: 6.0 }, ..Default::default() }
    }

    fn check(kind: UltKind, o: &Observation, airborne: bool) -> bool {
        let (m, t) = (model(), terrain());
        let cx = UltContext { jump: &m, terrain: &t, ride_height: 1.2, stomp_dx: 0.5, stomp_dy: (0.6, 1.6) };
        wants_ult(kind, o, airborne, &cx)
    }

    #[test]
    fn kinds_by_character() {
        assert_eq!(ult_kind("cubu"), UltKind::Instant);
        assert_eq!(ult_kind("maiden"), UltKind::Instant);
        assert_eq!(ult_kind("meme"), UltKind::Instant);
        assert_eq!(ult_kind("hitsuji"), UltKind::AirJump);
        assert_eq!(ult_kind("zero"), UltKind::None);
        assert_eq!(ult_kind("okuda"), UltKind::None);
    }

    #[test]
    fn instant_fires_only_when_full() {
        let me = body(0.0, -2.8);
        assert!(check(UltKind::Instant, &obs(me, body(3.0, -2.8)), false));
        let half = BodyObs { ult: 15.0, ..me };
        assert!(!check(UltKind::Instant, &obs(half, body(3.0, -2.8)), false));
        // 満タン値 0（シミュレーターや okuda）では撃たない
        let none = BodyObs { ult: 0.0, ult_max: 0.0, ..me };
        assert!(!check(UltKind::Instant, &obs(none, body(3.0, -2.8)), false));
        assert!(!check(UltKind::None, &obs(me, body(3.0, -2.8)), false));
    }

    #[test]
    fn air_jump_saves_a_fall_off_the_edge() {
        // 床の外（x = 7.5）を落ちている
        let falling = BodyObs { vel: Vec2 { x: 2.0, y: -3.0 }, ..body(7.5, -2.0) };
        let o = obs(falling, body(-3.0, -2.8));
        assert!(check(UltKind::AirJump, &o, true));
        // 地面にいるとき・上っているとき・逆さまのときは撃たない
        assert!(!check(UltKind::AirJump, &o, false));
        let rising = BodyObs { vel: Vec2 { x: 2.0, y: 3.0 }, ..falling };
        assert!(!check(UltKind::AirJump, &obs(rising, body(-3.0, -2.8)), true));
        let flipped = BodyObs { angle_deg: 120.0, ..falling };
        assert!(!check(UltKind::AirJump, &obs(flipped, body(-3.0, -2.8)), true));
    }

    #[test]
    fn air_jump_stomps_from_above() {
        // 床の上を下りている。撃つと上へ伸びて、少し先の相手の真上を下りながら通る
        let me = BodyObs { vel: Vec2 { x: 2.5, y: -1.0 }, ..body(-2.0, -1.0) };
        let path = air_jump_path(&model(), &me, 100);
        let opp_x = path.iter().zip(path.iter().skip(1)).find(|(a, b)| b[1] < a[1] && b[1] < -1.0).map(|(_, b)| b[0]).unwrap();
        let opp = body(opp_x, -1.0 - 1.0);
        assert!(check(UltKind::AirJump, &obs(me, opp), true));
        // 相手が遠く離れていれば、床に着地できるので撃たない
        assert!(!check(UltKind::AirJump, &obs(me, body(5.5, -2.8)), true));
    }

    #[test]
    fn air_jump_does_not_rise_into_the_opponent() {
        // 相手が真上近くにいる：上りで相手の板に胴体が当たるので撃たない
        let me = BodyObs { vel: Vec2 { x: 0.0, y: -0.5 }, ..body(0.0, -1.5) };
        assert!(!check(UltKind::AirJump, &obs(me, body(0.2, -0.5)), true));
    }
}
