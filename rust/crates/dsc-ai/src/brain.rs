//! 対戦用のCPU（仕様書「CPUの構成」）。状況を読み、行動を決め、操作に変える。
//!
//! ダメージは「相手の板が自分の胴体に当たったとき」だけ入る。攻めは相手の上から板で乗ること、
//! 守りは相手の真下にいないこと。行動は次の優先順で決める：
//! 1. 空中：着地点の傾きに板を合わせる（E3）
//! 2. 倒れている：復帰処理（E2）。寝ているときは車輪が着かないので、接地カウントでは空中と区別できない。
//!    真下の床からの高さが車輪の高さより十分低く、上下に動いていなければ「地面にいる」とみなす
//! 3. 場外回避：境界の手前では、内側へ向かう
//! 4. 防御：相手が空中で、自分の近くに降りてきそうなら横へ逃げる
//! 5. 狙って跳ぶ：ジャンプを溜めながら相手の方へ板を傾け、今離せば下りの途中で相手の真上を通ると
//!    予測できた瞬間に離す（E4 の予測）。相手が遠ければ、傾ききったところで離して近づく。
//!    R2500 は中央の丘（約42°の斜面）を走って登れないので、移動もジャンプで行う
//!
//! ウルトは行動とは別に、毎ステップ撃つかを決めて操作に足す（ult.rs）
//!
//! 重力が上向きのとき（Rugby の上半分。StageGravity）は、世界を原点のまわりに180°回して、
//! 重力が下向きの世界として判断する。180°の回転は左右の向きも回転の向きも変えないので、
//! 自分の向き（facing）を反転させるだけで、出した操作（前後・回転）はそのままゲームに使える。
//! 重力が切り替わるのは y = 0 の線で、回した世界でも同じ線になる。相手がその線の向こう（上）にいるときは、
//! 線を越えられる場所（床がいちばん高く、立っていられる傾きのところ）へ行き、越えられる予測ができたら跳ぶ

use serde::{Deserialize, Serialize};

use crate::jump::{JumpModel, predict};
use crate::landing::{LandingParams, Wheels, landing_rotate, landing_target, rotate_toward};
use crate::recovery::{Recovery, RecoveryParams};
use crate::terrain::Terrain;
use crate::ult::{UltContext, UltKind, wants_ult};
use crate::{BodyObs, Input, Observation, Policy, Vec2};

/// キャラごとの調整済みの値と形
#[derive(Debug, Clone, Copy)]
pub struct Knowledge {
    pub recovery: RecoveryParams,
    pub landing: LandingParams,
    pub wheels: Wheels,
    pub jump: JumpModel,
}

/// 行動の判断に使う数値（E5・E6 で調整する）
/// 設定ファイルに無い項目は仮の値（Default）で補う。項目を足しても、前に保存した値をそのまま読める
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Tactics {
    /// これより傾いていたら倒れているとみなす（度）
    pub upright_deg: f32,
    /// 防御：相手の予測着地点が、自分からこの距離以内なら逃げる
    pub dodge_radius: f32,
    /// 踏みつけ：相手の真上（横の差がこの値以内）を通る予測なら離す
    pub stomp_dx: f32,
    /// 踏みつけ：相手の中心より、この高さの範囲を下りながら通る予測なら離す
    pub stomp_dy: (f32, f32),
    /// ジャンプを溜める最短・最長のステップ数（1だと姿勢の切り替えで接地が外れて跳べない）
    pub min_hold: u32,
    pub max_hold: u32,
    /// 溜めながら相手の方へ傾ける角度：相手との横の距離 1 あたりの度数と、その上限
    pub tilt_per_unit: f32,
    pub max_tilt_deg: f32,
    /// 相手がこれより遠ければ、傾ききったところで離して近づく
    pub travel_dx: f32,
    /// 場外の境界から、この距離より内側にいる
    pub bounds_margin: f32,
    /// 床からの高さが「車輪の高さ × この値」より低く、上下の速さが on_ground_vy 未満なら地面にいる
    pub on_ground_height: f32,
    pub on_ground_vy: f32,
}

impl Default for Tactics {
    fn default() -> Self {
        Self {
            upright_deg: 45.0,
            dodge_radius: 1.3,
            stomp_dx: 0.5,
            stomp_dy: (0.6, 1.6),
            min_hold: 2,
            max_hold: 25,
            tilt_per_unit: 8.0,
            max_tilt_deg: 30.0,
            travel_dx: 3.0,
            bounds_margin: 1.0,
            on_ground_height: 0.8,
            on_ground_vy: 1.0,
        }
    }
}

/// いま取っている行動（調査用）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Land,
    Recover,
    StayInBounds,
    Dodge,
    /// 狙って跳ぶ（溜めて、離す）
    Aim,
}

pub struct Brain {
    knowledge: Knowledge,
    tactics: Tactics,
    /// いまの向き（flipped）で見た地形と、180°回した地形
    terrain: Terrain,
    other_terrain: Terrain,
    /// 重力が上向きで、世界を180°回して見ているか
    flipped: bool,
    /// 重力の反転を一度でも見たか（重力反転のあるステージか）
    seen_flip: bool,
    /// 相手が天井に逆さまで立ち続けているステップ数（R2500 の天井に一瞬ぶつかったのと区別する）
    opp_on_ceiling_steps: u32,
    recovery: Recovery,
    /// ジャンプを溜めているステップ数（溜めていなければ None）
    charging: Option<u32>,
    last_action: Action,
    ult: UltKind,
    /// 前のステップでウルトを押したか、そのときのゲージ、出したジャンプの押し方
    ult_pressed: bool,
    last_gauge: f32,
    last_jump: bool,
    /// ウルトの効果（zero の停止・nagisa の突進）が続く残りステップ数。この間は回転も移動も入れない
    ult_active: u32,
}

/// 予測する最大ステップ数（2秒）
const LOOKAHEAD: usize = 100;
/// 境目を越えに跳ぶ場所を探すときの、片側の分割数
const CROSSING_SAMPLES: usize = 40;
/// 境目（y = 0）をこの高さだけ越えれば、向こうの重力に引かれる
const CROSSING_MARGIN: f32 = 0.3;
/// 相手が天井に逆さまでこのステップ数（0.5秒）立ち続けたら、向こう側の重力が逆とみなす
const OPP_ON_CEILING_STEPS: u32 = 25;

impl Brain {
    pub fn new(knowledge: Knowledge, tactics: Tactics, terrain: Terrain) -> Self {
        Self {
            recovery: Recovery::new(knowledge.recovery),
            knowledge,
            tactics,
            other_terrain: terrain.rotated_half_turn(),
            terrain,
            flipped: false,
            seen_flip: false,
            opp_on_ceiling_steps: 0,
            charging: None,
            last_action: Action::Aim,
            ult: UltKind::None,
            ult_pressed: false,
            last_gauge: 0.0,
            last_jump: false,
            ult_active: 0,
        }
    }

    /// ウルトの使い方を設定する（既定は撃たない）
    pub fn with_ult(mut self, ult: UltKind) -> Self {
        self.ult = ult;
        self
    }

    pub fn last_action(&self) -> Action {
        self.last_action
    }

    fn decide(&mut self, obs: &Observation) -> (Action, Input) {
        let (me, opp) = (&obs.me, &obs.opp);
        let k = self.knowledge;
        let gravity_y = k.jump.gravity_y;
        let mut input = Input::default();

        // 1. 空中
        if me.ground_count == 0 && !self.lying_on_ground(me) {
            self.charging = None;
            self.recovery.reset();
            input.rotate = landing_rotate(&k.landing, me, &self.terrain, gravity_y, k.wheels);
            return (Action::Land, input);
        }

        // 2. 倒れている
        if me.angle_deg.abs() > self.tactics.upright_deg {
            self.charging = None;
            input.rotate = self.recovery.update(me);
            return (Action::Recover, input);
        }
        self.recovery.reset();

        // 3. 場外回避
        let limit = obs.bounds.x.abs() - self.tactics.bounds_margin;
        if obs.bounds.x != 0.0 && me.pos.x.abs() > limit {
            self.charging = None;
            input.move_x = drive(me, -me.pos.x.signum());
            return (Action::StayInBounds, input);
        }

        // 4. 防御：相手が上から降りてくる（溜めていたら離すので、跳んで逃げることになる）
        if opp.ground_count == 0 && opp.pos.y > me.pos.y {
            let land_x = self
                .terrain
                .predict_landing([opp.pos.x, opp.pos.y], [opp.vel.x, opp.vel.y], gravity_y, k.wheels.ride_height, 3.0)
                .map_or(opp.pos.x, |l| l.x);
            if (land_x - me.pos.x).abs() < self.tactics.dodge_radius {
                self.charging = None;
                let away = if me.pos.x == land_x { -me.pos.x.signum() } else { (me.pos.x - land_x).signum() };
                input.move_x = drive(me, away);
                return (Action::Dodge, input);
            }
        }

        // 5. 狙って跳ぶ。相手が重力の境目（y = 0）の向こうにいれば、境目を越えに行く
        let t = self.tactics;
        let held = self.charging.map_or(1, |h| h + 1);
        let cross = self.across_gravity_line(opp);
        let target_x = if cross { self.crossing_x(me, obs) } else { opp.pos.x };
        let dx = target_x - me.pos.x;
        let tilt = -dx.signum() * (t.tilt_per_unit * dx.abs()).min(t.max_tilt_deg);
        input.rotate = rotate_toward(&k.landing, me, tilt);
        input.move_x = drive(me, dx.signum());
        let release = held > t.min_hold
            && (if cross { self.crossing_predicted(me) } else { self.stomp_predicted(me, opp) }
                || held >= t.max_hold
                || (dx.abs() > t.travel_dx && (me.angle_deg - tilt).abs() < 3.0));
        input.jump = !release;
        self.charging = if release { None } else { Some(held) };
        (Action::Aim, input)
    }

    /// 車輪は着いていないが、地面に寝ている（真下の床が近く、上下に動いていない）
    fn lying_on_ground(&self, me: &BodyObs) -> bool {
        let Some((floor, _)) = self.terrain.floor_below(me.pos.x, me.pos.y + 0.1) else { return false };
        let height = me.pos.y - floor;
        height < self.knowledge.wheels.ride_height * self.tactics.on_ground_height && me.vel.y.abs() < self.tactics.on_ground_vy
    }

    /// 相手が重力の境目（y = 0）の向こうにいるか。重力の反転を自分で見る前でも、相手が境目の向こうで
    /// 天井に逆さまで立ち続けていれば（向こう側の重力が逆）そうみなす
    fn across_gravity_line(&self, opp: &BodyObs) -> bool {
        opp.pos.y > 0.0 && (self.seen_flip || self.opp_on_ceiling_steps >= OPP_ON_CEILING_STEPS)
    }

    /// 相手が天井に逆さまで立っているか。天井に車輪で立っていれば、体の中心から天井までは車輪の高さくらい。
    /// 寝ている相手が地面にめり込んで、すぐ上の地面の面を拾わないよう、近すぎる面は除く
    fn opp_on_ceiling(&self, opp: &BodyObs) -> bool {
        if opp.pos.y <= 0.0 {
            return false;
        }
        // 180°回した地形で相手の真下を見ると、いまの向きでの相手の真上（天井）になる。
        let ride = self.knowledge.wheels.ride_height;
        let on_ceiling = self
            .other_terrain
            .floor_below(-opp.pos.x, -opp.pos.y)
            .is_some_and(|(floor, _)| (ride * 0.5..ride * 1.5).contains(&(-opp.pos.y - floor)));
        opp.ground_count > 0 && opp.angle_deg.abs() > 135.0 && on_ceiling
    }

    /// 境目を越えに跳ぶ場所の x：自分の側（左右）で、立っていられる傾きのうち床がいちばん高いところ
    fn crossing_x(&self, me: &BodyObs, obs: &Observation) -> f32 {
        let limit = if obs.bounds.x != 0.0 { obs.bounds.x.abs() - self.tactics.bounds_margin } else { 10.0 };
        let side = if me.pos.x != 0.0 { me.pos.x.signum() } else { 1.0 };
        let mut best = (f32::NEG_INFINITY, me.pos.x);
        for i in 0..=CROSSING_SAMPLES {
            let x = side * limit * i as f32 / CROSSING_SAMPLES as f32;
            if let Some((floor, angle)) = self.terrain.floor_below(x, -0.05)
                && angle.abs() <= self.tactics.upright_deg
                && floor > best.0
            {
                best = (floor, x);
            }
        }
        best.1
    }

    /// 今跳んだら、境目を十分に越えるか
    fn crossing_predicted(&self, me: &BodyObs) -> bool {
        predict(&self.knowledge.jump, me, LOOKAHEAD).iter().any(|p| p[1] > CROSSING_MARGIN)
    }

    /// 今跳んだら、下りの途中で相手の真上を通るか
    fn stomp_predicted(&self, me: &BodyObs, opp: &BodyObs) -> bool {
        let path = predict(&self.knowledge.jump, me, LOOKAHEAD);
        let (lo, hi) = self.tactics.stomp_dy;
        path.windows(2).any(|w| {
            let descending = w[1][1] < w[0][1];
            let p = w[1];
            descending && (p[0] - opp.pos.x).abs() < self.tactics.stomp_dx && (lo..=hi).contains(&(p[1] - opp.pos.y))
        })
    }

    /// 空中で狙う角度（調査用）
    pub fn landing_target(&self, obs: &Observation) -> f32 {
        landing_target(&obs.me, &self.terrain, self.knowledge.jump.gravity_y, self.knowledge.wheels)
    }
}

impl Policy for Brain {
    fn act(&mut self, obs: &Observation) -> Input {
        let flipped = obs.gravity_up.y < 0.0;
        self.seen_flip |= flipped;
        if flipped != self.flipped {
            // 上下が入れ替わったら、地形を取り替え、溜めと起き上がりの途中経過を捨てる
            self.flipped = flipped;
            std::mem::swap(&mut self.terrain, &mut self.other_terrain);
            self.charging = None;
            self.recovery.reset();
        }
        // 体の正面の向きは、ワールドでの P1・P2 で決まる（回した世界の facing とは符号が逆になる）
        let nose = obs.me.facing;
        let view = if flipped { half_turn(obs) } else { *obs };
        let obs = &view;
        self.opp_on_ceiling_steps = if self.opp_on_ceiling(&obs.opp) { self.opp_on_ceiling_steps + 1 } else { 0 };

        // 前のステップで押したウルトが出た（ゲージが減った）なら、効果が続く間は待つ。
        // ジャンプの押し方は変えない（溜めていたのを離すと、その場で跳んでしまう）
        if self.ult_pressed && obs.me.ult + 0.5 < self.last_gauge {
            self.ult_active = self.ult.active_steps();
        }
        self.last_gauge = obs.me.ult;
        self.ult_pressed = false;
        if self.ult_active > 0 {
            self.ult_active -= 1;
            return Input { jump: self.last_jump, ..Default::default() };
        }

        let (action, mut input) = self.decide(obs);
        self.last_action = action;
        let cx = UltContext {
            jump: &self.knowledge.jump,
            terrain: &self.terrain,
            ride_height: self.knowledge.wheels.ride_height,
            stomp_dx: self.tactics.stomp_dx,
            stomp_dy: self.tactics.stomp_dy,
            dodge_radius: self.tactics.dodge_radius,
            bounds_margin: self.tactics.bounds_margin,
            nose,
        };
        // 「空中」の行動は、空中にいて地面に寝てもいないときだけ選ばれる
        input.ult = wants_ult(self.ult, obs, action == Action::Land, &cx);
        self.ult_pressed = input.ult;
        self.last_jump = input.jump;
        input
    }
}

/// 観測を原点のまわりに180°回す。自分と相手の位置・速度を反転し、角度に180°を足し、向き（facing）を反転する
fn half_turn(obs: &Observation) -> Observation {
    fn turn(b: &BodyObs) -> BodyObs {
        let mut angle = b.angle_deg + 180.0;
        if angle > 180.0 {
            angle -= 360.0;
        }
        BodyObs {
            pos: Vec2 { x: -b.pos.x, y: -b.pos.y },
            vel: Vec2 { x: -b.vel.x, y: -b.vel.y },
            angle_deg: angle,
            facing: -b.facing,
            ..*b
        }
    }
    Observation { me: turn(&obs.me), opp: turn(&obs.opp), gravity_up: Vec2 { x: -obs.gravity_up.x, y: -obs.gravity_up.y }, ..*obs }
}

/// ワールドの向き（+1 で右）に走る入力。P2 は左右反転しているので向きが逆
fn drive(me: &BodyObs, world_dir: f32) -> f32 {
    let facing = if me.facing == 0.0 { 1.0 } else { me.facing };
    world_dir * facing
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brain() -> Brain {
        let knowledge = Knowledge {
            recovery: RecoveryParams::default(),
            landing: LandingParams::default(),
            wheels: Wheels { ride_height: 1.2 },
            jump: JumpModel { jump_force: 9.0, max_speed: 5.0, deceleration: 1.0, center_of_mass: [0.0, -0.5], gravity_y: -9.81, dt: 0.02 },
        };
        let floor: [[f32; 2]; 4] = [[-10.0, -5.0], [10.0, -5.0], [10.0, -4.0], [-10.0, -4.0]];
        Brain::new(knowledge, Tactics::default(), Terrain::from_polygons([&floor[..]]))
    }

    fn standing(x: f32, facing: f32) -> BodyObs {
        BodyObs { pos: Vec2 { x, y: -2.8 }, ground_count: 2, facing, ..Default::default() }
    }

    fn obs(me: BodyObs, opp: BodyObs) -> Observation {
        Observation { me, opp, gravity_up: Vec2 { x: 0.0, y: 1.0 }, bounds: Vec2 { x: 9.0, y: 5.0 }, ..Default::default() }
    }

    #[test]
    fn aims_toward_the_opponent() {
        let mut b = brain();
        // P1 は右の相手へ：前進しながら溜め、右へ傾ける（角度を負に）
        let input = b.act(&obs(standing(-4.0, 1.0), standing(4.0, -1.0)));
        assert_eq!(b.last_action(), Action::Aim);
        assert!(input.jump && input.move_x == 1.0 && input.rotate == -1.0, "{input:?}");
        // P2（左右反転）は左の相手へ：move_x はやはり前進（+1）、左へ傾ける（角度を正に）
        let mut b = brain();
        let input = b.act(&obs(standing(4.0, -1.0), standing(-4.0, 1.0)));
        assert!(input.jump && input.move_x == 1.0 && input.rotate == 1.0, "{input:?}");
    }

    #[test]
    fn releases_when_tilted_toward_a_far_opponent() {
        let mut b = brain();
        let tilted = BodyObs { angle_deg: -30.0, ..standing(-4.0, 1.0) };
        let presses: Vec<bool> = (0..4).map(|_| b.act(&obs(tilted, standing(4.0, -1.0))).jump).collect();
        // 最短の溜め（2ステップ）の後、傾ききっているので離す
        assert_eq!(presses, [true, true, false, true]);
    }

    #[test]
    fn dodges_an_opponent_landing_on_top() {
        let mut b = brain();
        let mut opp = standing(-3.9, -1.0);
        opp.pos.y = 0.0;
        opp.ground_count = 0;
        let input = b.act(&obs(standing(-4.0, 1.0), opp));
        assert_eq!(b.last_action(), Action::Dodge);
        assert_eq!(input.move_x, -1.0);
    }

    #[test]
    fn recovers_when_lying_on_the_ground() {
        let mut b = brain();
        // 逆さまに寝ている：車輪は着いていないが、床（y = -4）のすぐ上で止まっている
        let me = BodyObs { angle_deg: 160.0, ground_count: 0, pos: Vec2 { x: -4.0, y: -3.7 }, facing: 1.0, ..Default::default() };
        b.act(&obs(me, standing(4.0, -1.0)));
        assert_eq!(b.last_action(), Action::Recover);
        // 同じ角度でも高いところにいれば空中
        let me = BodyObs { pos: Vec2 { x: -4.0, y: 0.0 }, ..me };
        b.act(&obs(me, standing(4.0, -1.0)));
        assert_eq!(b.last_action(), Action::Land);
    }

    #[test]
    fn plays_upside_down_when_gravity_points_up() {
        // 床（y = -5..-4）を180°回すと天井（y = 4..5）。重力が上向きで、天井に車輪で立っている（逆さま）
        let on_ceiling = |x: f32, facing: f32| BodyObs { pos: Vec2 { x, y: 2.8 }, angle_deg: 180.0, ground_count: 2, facing, ..Default::default() };
        let mut o = obs(on_ceiling(4.0, 1.0), on_ceiling(-4.0, -1.0));
        o.gravity_up = Vec2 { x: 0.0, y: -1.0 };
        let mut b = brain();
        let input = b.act(&o);
        // 倒れているとは見なさず、相手を狙う。相手はワールドの左なので、P1 は後ろ（move_x = -1）へ走る
        assert_eq!(b.last_action(), Action::Aim);
        assert!(input.jump && input.move_x == -1.0, "{input:?}");
        // 重力が下向きのままなら、同じ姿勢は倒れている
        o.gravity_up = Vec2 { x: 0.0, y: 1.0 };
        b.act(&o);
        assert_eq!(b.last_action(), Action::Recover);
    }

    #[test]
    fn heads_for_the_gravity_line_when_the_opponent_is_across_it() {
        // お椀形の床：中央は低く（y = -6）、端（|x| = 6..8）は高い（y = -1.5、平ら）
        let low: [[f32; 2]; 4] = [[-6.0, -7.0], [6.0, -7.0], [6.0, -6.0], [-6.0, -6.0]];
        let bottom = |x: f32| BodyObs { pos: Vec2 { x, y: -4.8 }, ..standing(x, 1.0) };
        let left: [[f32; 2]; 4] = [[-8.0, -5.0], [-6.0, -5.0], [-6.0, -1.5], [-8.0, -1.5]];
        let right: [[f32; 2]; 4] = [[6.0, -5.0], [8.0, -5.0], [8.0, -1.5], [6.0, -1.5]];
        // 天井：y = 4..5
        let ceiling: [[f32; 2]; 4] = [[-8.0, 4.0], [8.0, 4.0], [8.0, 5.0], [-8.0, 5.0]];
        let mut b = Brain::new(brain().knowledge, Tactics::default(), Terrain::from_polygons([&low[..], &left[..], &right[..], &ceiling[..]]));
        // 相手は境目の向こうの天井に逆さまで立っている。自分は中央の床（x = 2）。
        // 立ち始めはふつうに相手（左）を狙い、0.5 秒立ち続けたら、右の高い床へ境目を越えに行く
        let opp = BodyObs { pos: Vec2 { x: -3.0, y: 2.8 }, angle_deg: 180.0, ground_count: 2, facing: -1.0, ..Default::default() };
        assert!(b.opp_on_ceiling(&opp));
        let first = b.act(&obs(bottom(2.0), opp));
        assert_eq!(first.move_x, -1.0, "{first:?}");
        let input = (1..OPP_ON_CEILING_STEPS).map(|_| b.act(&obs(bottom(2.0), opp))).last().unwrap();
        assert_eq!(b.last_action(), Action::Aim);
        assert_eq!(input.move_x, 1.0, "{input:?}");
        // 高い床（y = -1.5 の上、体の中心 -0.3）からなら、跳べば境目を越える
        let high = BodyObs { pos: Vec2 { x: 7.0, y: -0.3 }, ..standing(7.0, 1.0) };
        assert!(b.crossing_predicted(&high));
        // 中央の床からでは届かない（跳び上がるのは約 4.1）
        assert!(!b.crossing_predicted(&bottom(2.0)));
        // 浮いている相手、天井の無いステージで逆さまの相手は、天井に立っているとみなさない
        assert!(!b.opp_on_ceiling(&BodyObs { ground_count: 0, angle_deg: 0.0, ..opp }));
        assert!(!brain().opp_on_ceiling(&opp));
    }

    #[test]
    fn holds_still_while_the_ult_is_active() {
        // zero：正面の相手に撃つ。出た（ゲージが 0 になった）ら、止まっている間は回転も移動も入れない
        let mut b = brain().with_ult(UltKind::Shoot);
        let full = BodyObs { hp: 3.0, ult: 170.0, ult_max: 170.0, ..standing(-2.0, 1.0) };
        let first = b.act(&obs(full, standing(2.0, -1.0)));
        assert!(first.ult && first.jump, "{first:?}");
        let fired = BodyObs { ult: 0.0, ..full };
        let held: Vec<Input> = (0..UltKind::Shoot.active_steps()).map(|_| b.act(&obs(fired, standing(2.0, -1.0)))).collect();
        assert!(held.iter().all(|i| *i == Input { jump: true, ..Default::default() }), "{held:?}");
        // 終わったら、ふつうに戻る
        let after = b.act(&obs(fired, standing(2.0, -1.0)));
        assert!(after.move_x != 0.0, "{after:?}");
    }

    #[test]
    fn fires_the_ult_only_when_set_up_and_full() {
        let full = BodyObs { hp: 3.0, ult: 100.0, ult_max: 100.0, ..standing(-4.0, 1.0) };
        // 既定（シミュレーターの E5 など）は撃たない
        let mut b = brain();
        assert!(!b.act(&obs(full, standing(4.0, -1.0))).ult);
        let mut b = brain().with_ult(UltKind::Instant);
        let input = b.act(&obs(full, standing(4.0, -1.0)));
        // 行動はそのまま、ウルトだけ足す
        assert!(input.ult && input.jump && input.move_x == 1.0, "{input:?}");
        let half = BodyObs { ult: 50.0, ..full };
        assert!(!b.act(&obs(half, standing(4.0, -1.0))).ult);
    }
}
