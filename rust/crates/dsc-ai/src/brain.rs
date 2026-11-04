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

use crate::jump::{JumpModel, predict};
use crate::landing::{LandingParams, Wheels, landing_rotate, landing_target, rotate_toward};
use crate::recovery::{Recovery, RecoveryParams};
use crate::terrain::Terrain;
use crate::{BodyObs, Input, Observation, Policy};

/// キャラごとの調整済みの値と形
#[derive(Debug, Clone, Copy)]
pub struct Knowledge {
    pub recovery: RecoveryParams,
    pub landing: LandingParams,
    pub wheels: Wheels,
    pub jump: JumpModel,
}

/// 行動の判断に使う数値（E5・E6 で調整する）
#[derive(Debug, Clone, Copy, PartialEq)]
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
    terrain: Terrain,
    recovery: Recovery,
    /// ジャンプを溜めているステップ数（溜めていなければ None）
    charging: Option<u32>,
    last_action: Action,
}

/// 予測する最大ステップ数（2秒）
const LOOKAHEAD: usize = 100;

impl Brain {
    pub fn new(knowledge: Knowledge, tactics: Tactics, terrain: Terrain) -> Self {
        Self { recovery: Recovery::new(knowledge.recovery), knowledge, tactics, terrain, charging: None, last_action: Action::Aim }
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

        // 5. 狙って跳ぶ
        let t = self.tactics;
        let held = self.charging.map_or(1, |h| h + 1);
        let dx = opp.pos.x - me.pos.x;
        let tilt = -dx.signum() * (t.tilt_per_unit * dx.abs()).min(t.max_tilt_deg);
        input.rotate = rotate_toward(&k.landing, me, tilt);
        input.move_x = drive(me, dx.signum());
        let release = held > t.min_hold
            && (self.stomp_predicted(me, opp)
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
        let (action, input) = self.decide(obs);
        self.last_action = action;
        input
    }
}

/// ワールドの向き（+1 で右）に走る入力。P2 は左右反転しているので向きが逆
fn drive(me: &BodyObs, world_dir: f32) -> f32 {
    let facing = if me.facing == 0.0 { 1.0 } else { me.facing };
    world_dir * facing
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vec2;

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
}
