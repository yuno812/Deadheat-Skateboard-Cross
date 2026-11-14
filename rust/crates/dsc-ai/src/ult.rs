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
//! - 狙い撃ち（zero）：0.5 秒止まってから、体の正面へ直進する弾を撃つ（2 ダメージ、地形はすり抜ける）。
//!   止まっている間は向きも固定なので、押す瞬間に「弾の線上に、そのときの相手が来る」と予測できたら撃つ。
//!   相手が上から降りてくるときは、止まると踏まれるので撃たない
//! - 突進（nagisa）：重力を切って、体の正面へ 0.4 秒・秒速 20 で進む。(1) 突進の途中で自分の板が相手の胴体に
//!   先に当たる（自分の胴体が相手の板に当たる前に）と予測できたら攻撃に、(2) 場外へ落ちそうなら復帰に使う。
//!   どちらも、突進の終わりと、そのあと残る速度で落ちる先が、場外の内側に収まるときだけ
//! - okuda はウルトなし
//!
//! シミュレーターはウルトを再現しないので、判断だけをここで行い、効果はゲームで確かめる。
//! 体の正面（transform.right）は、P1 は (cos, sin)、左右反転した P2 は (-cos, -sin)。
//! 重力反転で世界を180°回して見ているときも、ワールドでの P1・P2 の符号（nose）で求める

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
    /// 弾の線上に相手が来るときに撃つ（zero）
    Shoot,
    /// 板から相手の胴体に突っ込めるとき・場外へ落ちそうなときに突進する（nagisa）
    Dash,
}

impl UltKind {
    /// 撃ったあと、効果が続くステップ数。この間は回転も移動も入れずに待つ（止まる・突進の向きを崩さない）
    pub fn active_steps(self) -> u32 {
        match self {
            UltKind::Shoot => SHOOT_STOP_STEPS as u32 + 1,
            UltKind::Dash => DASH_STEPS as u32,
            _ => 0,
        }
    }
}

/// キャラ名（Prefab のファイル名の小文字）からウルトの使い方を決める
pub fn ult_kind(character: &str) -> UltKind {
    match character {
        "cubu" | "maiden" | "meme" => UltKind::Instant,
        "hitsuji" => UltKind::AirJump,
        "zero" => UltKind::Shoot,
        "nagisa" => UltKind::Dash,
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

/// zero（zeroult.cs と zero.prefab）：止まるステップ数（0.5 秒）、弾の速さ、弾が出る位置（体の中心から正面へ）
const SHOOT_STOP_STEPS: usize = 25;
const BULLET_SPEED: f32 = 18.0;
const BULLET_SPAWN: f32 = 1.0;
/// 弾の中心と、相手の体の縦の線（板から頭まで）の距離がこれ以内なら当たるとみなす。
/// 弾（zero_Bullet）は半径 0.65 と 0.49 の円、相手の胴体の半幅は約 0.45。外したくないので少し狭めに取る
const BULLET_HIT_DIST: f32 = 0.9;
/// これより先の当たりは、相手の動きの予測が当てにならないので狙わない（秒）
const SHOOT_MAX_S: f32 = 1.2;

/// nagisa（nagisault.cs と Nagisa.prefab）：突進の速さ、ステップ数（0.4 秒）、突進のあとに残る速度の割合
const DASH_SPEED: f32 = 20.0;
const DASH_STEPS: usize = 20;
const DASH_AFTER: f32 = 0.2;
/// 突進の当たりを調べる、1 ステップあたりの分割数（1 ステップで 0.4 進むので、0.1 ずつ見る）
const DASH_SUBSTEPS: usize = 4;
/// 突進の当たりの見積もりの余裕：相手の板はこれだけ広く、相手の胴体はこれだけ狭く見る
const DASH_MARGIN: f32 = 0.15;
/// 板が相手の胴体に当たる瞬間、自分の中心が相手の中心よりこれだけ高くなければ狙わない
const DASH_MIN_ABOVE: f32 = 0.3;
/// 突進の途中で地形にぶつかるかを調べるとき、体の中心のどれだけ上から真下を見るか
/// （丘の中に入った点でも、その上の面を拾える。天井（Rugby）までは届かない高さ）
const TERRAIN_LOOKUP: f32 = 2.5;

/// 体の中心から見た板と胴体の四角（書き出しの当たり判定の範囲。nagisa・zero でほぼ同じ）：中心の高さ、半幅、半分の高さ
const BOARD: (f32, f32, f32) = (-0.9, 0.88, 0.1);
const BODY: (f32, f32, f32) = (-0.22, 0.45, 0.58);

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
    /// 防御の範囲（Tactics の dodge_radius）と、場外の境界からの余裕（bounds_margin）
    pub dodge_radius: f32,
    pub bounds_margin: f32,
    /// ワールドでの P1（+1）・P2（-1）。体の正面の向きを決める
    pub nose: f32,
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
        UltKind::Shoot => wants_shoot(obs, cx),
        UltKind::Dash => wants_dash(obs, airborne, cx),
    }
}

/// このまま落ちると、場外の内側に着地しない
fn falls_out(obs: &Observation, cx: &UltContext) -> bool {
    let me = &obs.me;
    let landing = cx.terrain.predict_landing([me.pos.x, me.pos.y], [me.vel.x, me.vel.y], cx.jump.gravity_y, cx.ride_height, 3.0);
    !landing.is_some_and(|l| obs.bounds.x == 0.0 || l.x.abs() < obs.bounds.x.abs())
}

/// 体の正面（transform.right）の向き
fn nose_dir(me: &BodyObs, nose: f32) -> [f32; 2] {
    let a = me.angle_deg.to_radians();
    [nose * a.cos(), nose * a.sin()]
}

fn up_dir(b: &BodyObs) -> [f32; 2] {
    let a = b.angle_deg.to_radians();
    [-a.sin(), a.cos()]
}

/// t 秒後の相手の位置。地上なら今の速度のまま、空中なら放物線で落ち、床に着いたらそこで止まる
fn predict_opp(opp: &BodyObs, t: f32, cx: &UltContext) -> [f32; 2] {
    let (p, v) = ([opp.pos.x, opp.pos.y], [opp.vel.x, opp.vel.y]);
    if opp.ground_count > 0 || t <= 0.0 {
        return [p[0] + v[0] * t, p[1] + v[1] * t];
    }
    match cx.terrain.predict_landing(p, v, cx.jump.gravity_y, cx.ride_height, t) {
        Some(l) => [l.x, l.y],
        None => [p[0] + v[0] * t, p[1] + v[1] * t + 0.5 * cx.jump.gravity_y * t * t],
    }
}

/// 相手が上から、自分の近くへ降りてくる（止まると踏まれる）
fn threatened(obs: &Observation, cx: &UltContext) -> bool {
    let (me, opp) = (&obs.me, &obs.opp);
    if opp.ground_count > 0 || opp.pos.y <= me.pos.y {
        return false;
    }
    let land_x = cx
        .terrain
        .predict_landing([opp.pos.x, opp.pos.y], [opp.vel.x, opp.vel.y], cx.jump.gravity_y, cx.ride_height, 3.0)
        .map_or(opp.pos.x, |l| l.x);
    (land_x - me.pos.x).abs() < cx.dodge_radius * 1.5
}

/// zero：押してから止まる 0.5 秒のあと、正面へ飛ぶ弾が、そのときの相手の体に当たるか
fn wants_shoot(obs: &Observation, cx: &UltContext) -> bool {
    let (me, opp) = (&obs.me, &obs.opp);
    if threatened(obs, cx) {
        return false;
    }
    let d = nose_dir(me, cx.nose);
    let spawn = [me.pos.x + d[0] * BULLET_SPAWN, me.pos.y + d[1] * BULLET_SPAWN];
    let dt = cx.jump.dt;
    let stop = SHOOT_STOP_STEPS as f32 * dt;
    let up = up_dir(opp);
    let mut k = 0;
    loop {
        let flight = k as f32 * dt;
        let t = stop + flight;
        if t > SHOOT_MAX_S {
            return false;
        }
        let b = [spawn[0] + d[0] * BULLET_SPEED * flight, spawn[1] + d[1] * BULLET_SPEED * flight];
        // 弾は画面の外で消える。場外の境界を画面の代わりに使う
        if obs.bounds.x != 0.0 && (b[0].abs() > obs.bounds.x.abs() || b[1].abs() > obs.bounds.y.abs()) {
            return false;
        }
        let o = predict_opp(opp, t, cx);
        // 相手の体の縦の線：板（中心から 1.1 下）から頭（0.3 上）まで
        let lo = [o[0] - up[0] * 1.1, o[1] - up[1] * 1.1];
        let hi = [o[0] + up[0] * 0.3, o[1] + up[1] * 0.3];
        if distance_to_segment(b, lo, hi) < BULLET_HIT_DIST {
            return true;
        }
        k += 1;
    }
}

/// nagisa：攻撃か復帰に使えて、突進の後も場外に出ないか
fn wants_dash(obs: &Observation, airborne: bool, cx: &UltContext) -> bool {
    dash_is_safe(obs, cx) && (dash_hits(obs, cx) || (airborne && falls_out(obs, cx)))
}

/// 突進の終わりが境界の内側で、そのあと残る速度で落ちる先も内側に着地する
fn dash_is_safe(obs: &Observation, cx: &UltContext) -> bool {
    let me = &obs.me;
    let d = nose_dir(me, cx.nose);
    let len = DASH_SPEED * DASH_STEPS as f32 * cx.jump.dt;
    let end = [me.pos.x + d[0] * len, me.pos.y + d[1] * len];
    if obs.bounds.x != 0.0
        && (end[0].abs() > obs.bounds.x.abs() - cx.bounds_margin || end[1].abs() > obs.bounds.y.abs() - cx.bounds_margin)
    {
        return false;
    }
    let after = [d[0] * DASH_SPEED * DASH_AFTER, d[1] * DASH_SPEED * DASH_AFTER];
    let landing = cx.terrain.predict_landing(end, after, cx.jump.gravity_y, cx.ride_height, 3.0);
    landing.is_some_and(|l| obs.bounds.x == 0.0 || l.x.abs() < obs.bounds.x.abs() - cx.bounds_margin)
}

/// 突進の途中で、自分の板が相手の胴体に最初に当たるか。自分の胴体が相手の板に当たる（自分がダメージ）、
/// 胴体どうし・板どうしが先に当たる（はじかれるだけ）なら当たらないとする
fn dash_hits(obs: &Observation, cx: &UltContext) -> bool {
    let (me, opp) = (&obs.me, &obs.opp);
    let d = nose_dir(me, cx.nose);
    let n = DASH_STEPS * DASH_SUBSTEPS;
    let step = DASH_SPEED * cx.jump.dt / DASH_SUBSTEPS as f32;
    for i in 1..=n {
        let p = [me.pos.x + d[0] * step * i as f32, me.pos.y + d[1] * step * i as f32];
        let o = predict_opp(opp, i as f32 * cx.jump.dt / DASH_SUBSTEPS as f32, cx);
        let my_board = Obb::part(p, me.angle_deg, BOARD);
        let my_body = Obb::part(p, me.angle_deg, BODY);
        let opp_board = Obb::part(o, opp.angle_deg, BOARD);
        let opp_body = Obb::part(o, opp.angle_deg, BODY);
        // 相手の動きの予測は外れるので、まずい当たり（自分の胴体と相手の板）は広めに、
        // 良い当たり（自分の板と相手の胴体）は狭めに見る
        if my_body.overlaps(&opp_board.grown(DASH_MARGIN)) {
            return false;
        }
        if my_board.overlaps(&opp_body.grown(-DASH_MARGIN)) {
            // 下から上がって当てる形は、少しずれると胴体が相手の板に当たるので狙わない
            return p[1] - o[1] >= DASH_MIN_ABOVE;
        }
        if my_body.overlaps(&opp_body) || my_board.overlaps(&opp_board) {
            return false;
        }
        // 地形にぶつかると突進はそこで止まる（R2500 の中央の丘をはさんだ突進など）
        if hits_terrain(p, cx) {
            return false;
        }
    }
    false
}

/// 体の中心 p が、地形に車輪の高さより近い（めり込む）か。少し上から真下を見るので、坂や丘の中に入る点も見つかる
fn hits_terrain(p: [f32; 2], cx: &UltContext) -> bool {
    cx.terrain.floor_below(p[0], p[1] + TERRAIN_LOOKUP).is_some_and(|(floor, _)| p[1] - floor < cx.ride_height * 0.8)
}

fn distance_to_segment(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let ap = [p[0] - a[0], p[1] - a[1]];
    let len2 = ab[0] * ab[0] + ab[1] * ab[1];
    let t = if len2 > 0.0 { ((ap[0] * ab[0] + ap[1] * ab[1]) / len2).clamp(0.0, 1.0) } else { 0.0 };
    let q = [a[0] + ab[0] * t - p[0], a[1] + ab[1] * t - p[1]];
    (q[0] * q[0] + q[1] * q[1]).sqrt()
}

/// 回転した四角（体の板・胴体のおおよその形）
#[derive(Clone, Copy)]
struct Obb {
    center: [f32; 2],
    axes: [[f32; 2]; 2],
    half: [f32; 2],
}

impl Obb {
    /// 体の中心 pos、角度 angle_deg の体の一部（中心の高さ、半幅、半分の高さ）
    fn part(pos: [f32; 2], angle_deg: f32, (cy, hw, hh): (f32, f32, f32)) -> Self {
        let (s, c) = angle_deg.to_radians().sin_cos();
        let (right, up) = ([c, s], [-s, c]);
        Self { center: [pos[0] + up[0] * cy, pos[1] + up[1] * cy], axes: [right, up], half: [hw, hh] }
    }

    /// 四方に margin だけ広げた（負なら縮めた）四角
    fn grown(&self, margin: f32) -> Self {
        Self { half: [(self.half[0] + margin).max(0.0), (self.half[1] + margin).max(0.0)], ..*self }
    }

    /// 分離軸で重なりを調べる
    fn overlaps(&self, other: &Obb) -> bool {
        let dot = |a: [f32; 2], b: [f32; 2]| a[0] * b[0] + a[1] * b[1];
        let radius = |o: &Obb, axis: [f32; 2]| o.half[0] * dot(o.axes[0], axis).abs() + o.half[1] * dot(o.axes[1], axis).abs();
        let d = [other.center[0] - self.center[0], other.center[1] - self.center[1]];
        self.axes.iter().chain(other.axes.iter()).all(|&axis| dot(d, axis).abs() <= radius(self, axis) + radius(other, axis))
    }
}

fn wants_air_jump(obs: &Observation, cx: &UltContext) -> bool {
    let (me, opp) = (&obs.me, &obs.opp);
    if me.angle_deg.abs() > AIR_JUMP_UPRIGHT_DEG || me.vel.y >= 0.0 {
        return false;
    }
    // (1) 復帰：このまま落ちると、場外の内側に着地しない
    if falls_out(obs, cx) {
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
        let cx = UltContext {
            jump: &m,
            terrain: &t,
            ride_height: 1.2,
            stomp_dx: 0.5,
            stomp_dy: (0.6, 1.6),
            dodge_radius: 1.3,
            bounds_margin: 1.0,
            nose: o.me.facing,
        };
        wants_ult(kind, o, airborne, &cx)
    }

    /// 床（上面 y = -4）に立っている：体の中心は車輪の高さ 1.2 だけ上
    fn standing(x: f32, facing: f32) -> BodyObs {
        BodyObs { ground_count: 2, facing, ult: 170.0, ult_max: 170.0, ..body(x, -2.8) }
    }

    fn charged(b: BodyObs) -> BodyObs {
        BodyObs { ult: 100.0, ult_max: 100.0, ..b }
    }

    #[test]
    fn kinds_by_character() {
        assert_eq!(ult_kind("cubu"), UltKind::Instant);
        assert_eq!(ult_kind("maiden"), UltKind::Instant);
        assert_eq!(ult_kind("meme"), UltKind::Instant);
        assert_eq!(ult_kind("hitsuji"), UltKind::AirJump);
        assert_eq!(ult_kind("zero"), UltKind::Shoot);
        assert_eq!(ult_kind("nagisa"), UltKind::Dash);
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

    #[test]
    fn shoots_when_the_opponent_is_in_the_line_of_fire() {
        // 同じ床の上、正面 4 先に止まっている相手：水平な弾が当たる
        assert!(check(UltKind::Shoot, &obs(standing(-2.0, 1.0), standing(2.0, -1.0)), false));
        // P2（左右反転）は左が正面
        assert!(check(UltKind::Shoot, &obs(standing(2.0, -1.0), standing(-2.0, 1.0)), false));
        // 相手が背中側なら撃たない
        assert!(!check(UltKind::Shoot, &obs(standing(2.0, 1.0), standing(-2.0, -1.0)), false));
        // 相手がずっと上（弾が通るころも線より上）なら撃たない
        assert!(!check(UltKind::Shoot, &obs(standing(-2.0, 1.0), body(2.0, 3.0)), false));
        // 体を 30° 上へ向けていれば、斜め上の相手に当たる
        let aimed = BodyObs { angle_deg: 30.0, ..standing(-2.0, 1.0) };
        let high = BodyObs { ground_count: 2, ..body(2.0, -2.8 + 4.0 * 30f32.to_radians().tan()) };
        assert!(check(UltKind::Shoot, &obs(aimed, high), false));
    }

    #[test]
    fn shoot_leads_a_moving_opponent() {
        // こちらへ走ってくる相手：止まる 0.5 秒と弾の時間のあいだに近づいた位置で当たる
        let runner = BodyObs { vel: Vec2 { x: -3.0, y: 0.0 }, ..standing(4.0, -1.0) };
        assert!(check(UltKind::Shoot, &obs(standing(-2.0, 1.0), runner), false));
        // 空中を落ちてくる相手：止まっている間に床まで降りてきて、弾の線に入る
        let falling = BodyObs { vel: Vec2 { x: 0.0, y: -1.0 }, ..body(2.5, -1.0) };
        assert!(check(UltKind::Shoot, &obs(standing(-2.0, 1.0), falling), false));
    }

    #[test]
    fn does_not_shoot_when_about_to_be_stomped() {
        // 相手が真上から降りてくる：止まると踏まれる
        let above = BodyObs { vel: Vec2 { x: 0.0, y: -2.0 }, ..body(-1.8, 0.5) };
        assert!(!check(UltKind::Shoot, &obs(standing(-2.0, 1.0), above), false));
    }

    #[test]
    fn dashes_board_first_into_the_body() {
        // 自分が相手より 0.8 高い：板が相手の胴体に先に当たる
        let me = charged(BodyObs { ground_count: 0, ..body(-4.0, -2.0) });
        assert!(check(UltKind::Dash, &obs(me, standing(0.0, -1.0)), true));
        // 同じ高さ：板どうしが先に当たる
        let level = charged(standing(-4.0, 1.0));
        assert!(!check(UltKind::Dash, &obs(level, standing(0.0, -1.0)), false));
        // 低い：自分の胴体が相手の板に当たる
        let lower = BodyObs { pos: Vec2 { x: -4.0, y: -3.6 }, ..level };
        assert!(!check(UltKind::Dash, &obs(lower, standing(0.0, -1.0)), false));
        // 相手が遠い（突進の 8 では届かない）
        assert!(!check(UltKind::Dash, &obs(me, standing(5.9, -1.0)), true));
    }

    #[test]
    fn dash_is_stopped_by_a_hill_in_between() {
        // 床の中央に丘（x = -1..1、上面 y = -2）。丘の向こうの相手へ水平に突進すると、丘にぶつかって止まる
        let floor: [[f32; 2]; 4] = [[-6.0, -5.0], [6.0, -5.0], [6.0, -4.0], [-6.0, -4.0]];
        let hill: [[f32; 2]; 4] = [[-1.0, -4.0], [1.0, -4.0], [1.0, -2.0], [-1.0, -2.0]];
        let t = Terrain::from_polygons([&floor[..], &hill[..]]);
        let m = model();
        let me = charged(BodyObs { ground_count: 0, ..body(-4.0, -2.0) });
        let o = obs(me, standing(3.0, -1.0));
        let cx = UltContext {
            jump: &m,
            terrain: &t,
            ride_height: 1.2,
            stomp_dx: 0.5,
            stomp_dy: (0.6, 1.6),
            dodge_radius: 1.3,
            bounds_margin: 1.0,
            nose: 1.0,
        };
        assert!(!wants_ult(UltKind::Dash, &o, true, &cx));
        // 丘が無ければ当たる
        assert!(check(UltKind::Dash, &o, true));
    }

    #[test]
    fn dash_stays_in_bounds() {
        // 正面に相手がいても、突進の終わりが境界（9 - 余裕 1）を越えるなら突進しない
        let me = charged(BodyObs { ground_count: 0, ..body(2.0, -2.0) });
        assert!(!check(UltKind::Dash, &obs(me, standing(5.5, -1.0)), true));
    }

    #[test]
    fn dash_saves_a_fall_toward_the_stage() {
        // 床（x = -6..6）の右外を落ちていて、正面（P2 は左）が床を向いている：突進で床の上へ戻る
        let falling = charged(BodyObs { vel: Vec2 { x: 1.0, y: -3.0 }, facing: -1.0, ..body(7.5, -2.0) });
        assert!(check(UltKind::Dash, &obs(falling, body(-3.0, -2.8)), true));
        // 正面が外を向いていたら突進しない
        let outward = BodyObs { facing: 1.0, ..falling };
        assert!(!check(UltKind::Dash, &obs(outward, body(-3.0, -2.8)), true));
    }
}
