//! 1試合ぶんのシミュレーション。Box2D 2.4 系の物理エンジン（`Physics`）の上で `MovePlayer` の処理を移植する。
//!
//! 1ステップの処理順は仕様書「1ステップの処理順」のとおり（各プレイヤーはP1→P2の順）：
//! 場外判定 → ウルトゲージ → 入力 → 姿勢の切り替え → 移動・回転・ジャンプ → 物理ステップ → 接触イベント。
//!
//! Unityとの違いを吸収している点：
//! - Unityは接触をコライダー単位で通知し、Box2Dは凸分割後のフィクスチャ単位で通知する。
//!   コライダーの組ごとに触れているフィクスチャの組を数え、0↔1 の変化を Enter / Exit にする。
//! - Unityの慣性モーメントは重心まわり、Box2D の b2MassData::I は原点まわり。
//! - Unityの角速度は度で読み書きされる。毎ステップの度↔ラジアンの往復も同じ定数で再現する。

mod cpp;
mod physics;
mod rs;

use std::collections::{BTreeMap, HashMap};
use std::fmt;

pub use cpp::CppPhysics;
pub use physics::{BodySpec, BodyState, ColliderId, Geometry, Physics, Piece, polygon_is_valid};
pub use rs::RsPhysics;

use crate::export::{CharacterExport, MassProps, Pose, Shape, ShapeKind, StageExport, Stats, Vec2};

/// Unity の Mathf.Rad2Deg / Mathf.Deg2Rad
const RAD2DEG: f32 = 57.29578;
const DEG2RAD: f32 = 0.017_453_292;
const LINEAR_SLOP: f32 = 0.005;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PlayerInput {
    pub move_x: f32,
    pub rotate: f32,
    pub jump: bool,
}

/// 1プレイヤーのステップ後の状態（トレースの1行に対応）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snapshot {
    pub slot: u8,
    pub pos: Vec2,
    /// 剛体の角度（度、-180..180）
    pub angle_deg: f32,
    pub vel: Vec2,
    /// 度/秒
    pub ang_vel_deg: f32,
    pub ground: u32,
    /// 胴体（MainCollision）が地面に触れている数。ゲームのロジックにはない、実験の判定用
    pub body_ground: u32,
    /// 板（AttackCollision）が地面に触れている数。同じく実験の判定用
    pub deck_ground: u32,
    pub pose: Pose,
    pub hp: f32,
    pub ult: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SimError(pub String);

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SimError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tag {
    Ground,
    Tire,
    Main,
    Attack,
    Other,
}

/// そのコライダーのオブジェクトに付いている、当たったときの処理
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Handler {
    WheelGroundCheck,
    MainCollision,
    AttackCollision,
    None,
}

#[derive(Debug, Clone, Copy)]
struct Collider {
    /// プレイヤーの添字（0 = P1, 1 = P2）。ステージは None
    owner: Option<usize>,
    tag: Tag,
    handler: Handler,
}

/// MovePlayer の状態
struct Player<P: Physics> {
    slot: u8,
    body: P::Body,
    stats: Stats,
    mass: f32,
    mass_by_pose: BTreeMap<Pose, MassProps>,
    pieces_by_pose: BTreeMap<Pose, Vec<Piece>>,
    active_pose: Pose,
    fixtures: Vec<P::Fixture>,
    spawn: Vec2,
    out_of_area: Vec2,
    hp: f32,
    ult_gauge: f32,
    last_position: Vec2,
    ground_wheel_count: u32,
    body_ground_count: u32,
    deck_ground_count: u32,
    is_charging_jump: bool,
    /// MovePlayer.chargebool。次のステップの初めに姿勢へ反映される
    charge_flag: bool,
}

pub struct Match<P: Physics> {
    physics: P,
    colliders: Vec<Collider>,
    /// コライダーの組 → 触れているフィクスチャの組の数
    touching: HashMap<(ColliderId, ColliderId), u32>,
    events: Vec<(ColliderId, ColliderId, bool)>,
    players: [Player<P>; 2],
    dt: f32,
    velocity_iterations: i32,
    position_iterations: i32,
}

impl<P: Physics> Match<P> {
    pub fn new(stage: &StageExport, p1: &CharacterExport, p2: &CharacterExport) -> Result<Self, SimError> {
        let mut physics = P::new(stage.world.gravity);
        let mut colliders = Vec::new();

        // ステージ：静的な剛体1つに、コライダーごとのフィクスチャを付ける
        let ground = physics.create_body(&BodySpec {
            dynamic: false,
            pos: [0.0, 0.0],
            angle: 0.0,
            bullet: false,
            gravity_scale: 1.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
        });
        for piece in pieces(&stage.shapes, None, &mut colliders) {
            physics.add_fixture(&ground, &piece);
        }

        let a = build_player(&mut physics, stage, p1, 0, &mut colliders)?;
        let b = build_player(&mut physics, stage, p2, 1, &mut colliders)?;
        Ok(Self {
            physics,
            colliders,
            touching: HashMap::new(),
            events: Vec::new(),
            players: [a, b],
            dt: stage.world.dt,
            velocity_iterations: stage.world.velocity_iterations as i32,
            position_iterations: stage.world.position_iterations as i32,
        })
    }

    /// 最初のステップの前に状態を入れる（シナリオの init）。省略した位置と角度はそのまま、速度は0。
    pub fn apply_init(&mut self, slot: u8, pos: Option<Vec2>, angle_deg: Option<f32>, vel: Option<Vec2>, ang_vel_deg: Option<f32>) {
        let p = &mut self.players[slot as usize - 1];
        let current = self.physics.state(&p.body);
        let position = pos.unwrap_or(current.pos);
        let angle = angle_deg.map_or(current.angle, |a| a * DEG2RAD);
        self.physics.set_transform(&p.body, position, angle);
        self.physics.set_linear_velocity(&p.body, vel.unwrap_or([0.0, 0.0]));
        self.physics.set_angular_velocity(&p.body, ang_vel_deg.unwrap_or(0.0) * DEG2RAD);
        p.last_position = position;
    }

    pub fn step(&mut self, inputs: [PlayerInput; 2]) {
        for (i, input) in inputs.into_iter().enumerate() {
            self.out_of_area(i);
            self.accumulate_ult_gauge(i);
            self.apply_pose(i);
            if self.players[i].hp > 0.0 {
                self.handle_move(i, input);
                self.handle_jump(i, input);
            }
        }
        self.physics.step(self.dt, self.velocity_iterations, self.position_iterations);
        self.dispatch_contacts();
    }

    pub fn snapshot(&self, slot: u8) -> Snapshot {
        let p = &self.players[slot as usize - 1];
        let s = self.physics.state(&p.body);
        Snapshot {
            slot: p.slot,
            pos: s.pos,
            angle_deg: delta_angle(s.angle * RAD2DEG),
            vel: s.vel,
            ang_vel_deg: s.w * RAD2DEG,
            ground: p.ground_wheel_count,
            body_ground: p.body_ground_count,
            deck_ground: p.deck_ground_count,
            pose: p.active_pose,
            hp: p.hp,
            ult: p.ult_gauge,
        }
    }

    /// AABB が重なっているフィクスチャの組の数（速度の調査用）
    pub fn contact_count(&self) -> usize {
        self.physics.contact_count()
    }

    // ---- MovePlayer.FixedUpdate ----

    fn out_of_area(&mut self, i: usize) {
        let p = &mut self.players[i];
        let s = self.physics.state(&p.body);
        if s.pos[0].abs() >= p.out_of_area[0].abs() || s.pos[1].abs() >= p.out_of_area[1].abs() {
            p.hp -= 1.0;
            // transform.position の書き換えは位置だけが剛体に反映され、角度と角速度は残る
            self.physics.set_transform(&p.body, p.spawn, s.angle);
            self.physics.set_linear_velocity(&p.body, [0.0, 0.0]);
        }
    }

    fn accumulate_ult_gauge(&mut self, i: usize) {
        let opponent = self.physics.state(&self.players[1 - i].body).pos;
        let p = &mut self.players[i];
        let pos = self.physics.state(&p.body).pos;
        let movement = [pos[0] - p.last_position[0], pos[1] - p.last_position[1]];
        let dir = normalized([opponent[0] - pos[0], opponent[1] - pos[1]]);
        let approach = movement[0] * dir[0] + movement[1] * dir[1];
        if approach > 0.0 {
            p.ult_gauge = (p.ult_gauge + approach * p.stats.ult_gain_multiplier).clamp(0.0, p.stats.ult_amount);
        }
        p.last_position = pos;
    }

    /// 前のステップの chargebool に従って当たり判定を切り替える。
    /// 消した当たり判定の接触は、その場で Exit として処理する（Unity の callbacksOnDisable）。
    fn apply_pose(&mut self, i: usize) {
        let want = if self.players[i].charge_flag { Pose::Charge } else { Pose::Normal };
        if self.players[i].active_pose == want {
            return;
        }
        let p = &mut self.players[i];
        for fixture in std::mem::take(&mut p.fixtures) {
            self.physics.destroy_fixture(&p.body, fixture);
        }
        p.fixtures = p.pieces_by_pose[&want].iter().map(|piece| self.physics.add_fixture(&p.body, piece)).collect();
        p.active_pose = want;
        set_mass(&mut self.physics, &p.body, p.mass, p.mass_by_pose[&want]);
        self.dispatch_contacts();
    }

    fn handle_move(&mut self, i: usize, input: PlayerInput) {
        let p = &self.players[i];
        let s = &p.stats;
        let mut velocity = self.physics.state(&p.body).vel;
        let move_dir = if p.slot == 1 { input.move_x } else { -input.move_x };

        if p.ground_wheel_count > 0 {
            if move_dir.abs() > 0.0 {
                velocity[0] += move_dir * s.move_force * self.dt;
            } else {
                velocity[0] = move_towards(velocity[0], 0.0, s.deceleration * self.dt);
            }
        }
        velocity[0] = velocity[0].clamp(-s.max_speed, s.max_speed);
        self.physics.set_linear_velocity(&p.body, velocity);

        if input.rotate != 0.0 {
            self.physics.apply_torque(&p.body, input.rotate * s.rotation_torque / input.rotate.abs());
        }
        let w = (self.physics.state(&p.body).w * RAD2DEG).clamp(-s.max_rotation_speed, s.max_rotation_speed);
        self.physics.set_angular_velocity(&p.body, w * DEG2RAD);
    }

    fn handle_jump(&mut self, i: usize, input: PlayerInput) {
        let p = &mut self.players[i];
        if input.jump && !p.is_charging_jump {
            p.is_charging_jump = true;
            p.charge_flag = true;
        }
        if !input.jump && p.is_charging_jump {
            p.is_charging_jump = false;
            if p.ground_wheel_count > 0 {
                let s = self.physics.state(&p.body);
                let up = [-s.angle.sin(), s.angle.cos()];
                let jump = p.stats.jump_force;
                self.physics.set_linear_velocity(&p.body, [s.vel[0] + up[0] * jump, s.vel[1] + up[1] * jump]);
            }
            p.charge_flag = false;
        }
    }

    // ---- 接触イベント（WheelGroundCheck / MainCollision / AttackCollision） ----

    fn dispatch_contacts(&mut self) {
        self.physics.drain_contacts(&mut self.events);
        for (a, b, begin) in std::mem::take(&mut self.events) {
            let key = if a <= b { (a, b) } else { (b, a) };
            let count = self.touching.entry(key).or_insert(0);
            let entered = begin && *count == 0;
            if begin {
                *count += 1;
            } else {
                *count = count.saturating_sub(1);
            }
            let exited = !begin && *count == 0;
            if entered || exited {
                self.on_collision(a, b, entered);
                self.on_collision(b, a, entered);
            }
        }
    }

    fn on_collision(&mut self, me: ColliderId, other: ColliderId, enter: bool) {
        let mine = self.colliders[me.0 as usize];
        let theirs = self.colliders[other.0 as usize];
        let Some(owner) = mine.owner else { return };
        match (mine.handler, theirs.tag, enter) {
            (Handler::WheelGroundCheck, Tag::Ground, true) => self.players[owner].ground_wheel_count += 1,
            (Handler::WheelGroundCheck, Tag::Ground, false) => {
                let g = &mut self.players[owner].ground_wheel_count;
                *g = g.saturating_sub(1);
            }
            // ゲームのスクリプトは何もしない。実験で「車輪から着地したか」を見分けるためだけに数える
            (Handler::MainCollision, Tag::Ground, true) => self.players[owner].body_ground_count += 1,
            (Handler::MainCollision, Tag::Ground, false) => {
                let g = &mut self.players[owner].body_ground_count;
                *g = g.saturating_sub(1);
            }
            (Handler::AttackCollision, Tag::Ground, true) => self.players[owner].deck_ground_count += 1,
            (Handler::AttackCollision, Tag::Ground, false) => {
                let g = &mut self.players[owner].deck_ground_count;
                *g = g.saturating_sub(1);
            }
            (Handler::MainCollision, Tag::Main, true) => self.hit(owner, theirs.owner, 10.0, false),
            (Handler::MainCollision, Tag::Attack, true) => self.hit(owner, theirs.owner, 25.0, true),
            (Handler::AttackCollision, Tag::Main, true) => self.hit(owner, theirs.owner, 25.0, false),
            (Handler::AttackCollision, Tag::Attack, true) => self.hit(owner, theirs.owner, 10.0, false),
            _ => {}
        }
    }

    /// HitSame（speed 10）/ HitAttack（speed 25）：相手のルートから自分への向きに速度を置き換える
    fn hit(&mut self, me: usize, other: Option<usize>, speed: f32, damage: bool) {
        let Some(other) = other else { return };
        let mine = self.physics.state(&self.players[me].body).pos;
        let theirs = self.physics.state(&self.players[other].body).pos;
        let dir = normalized([mine[0] - theirs[0], mine[1] - theirs[1]]);
        self.physics.set_linear_velocity(&self.players[me].body, [dir[0] * speed, dir[1] * speed]);
        if damage {
            let attack = self.players[other].stats.attack;
            self.players[me].hp -= attack;
        }
    }
}

// ---- 組み立て ----

fn build_player<P: Physics>(
    physics: &mut P,
    stage: &StageExport,
    export: &CharacterExport,
    index: usize,
    colliders: &mut Vec<Collider>,
) -> Result<Player<P>, SimError> {
    let slot = index as u8 + 1;
    let data = export
        .slot(slot)
        .ok_or_else(|| SimError(format!("{}: slot {slot} がありません", export.character)))?;
    for pose in [Pose::Normal, Pose::Charge, Pose::Ult] {
        if !data.poses.contains_key(&pose) {
            return Err(SimError(format!("{}: 姿勢 {pose:?} の質量がありません", export.character)));
        }
    }

    let spawn = stage.spawn[index];
    let body = physics.create_body(&BodySpec {
        dynamic: true,
        pos: spawn,
        angle: data.body.angle_deg * DEG2RAD,
        // Unity の CollisionDetectionMode2D.Continuous は、動く剛体同士でも連続衝突判定をする
        bullet: data.body.collision_detection == "Continuous",
        gravity_scale: data.body.gravity_scale,
        linear_damping: data.body.linear_damping,
        angular_damping: data.body.angular_damping,
    });

    let mut pieces_by_pose = BTreeMap::new();
    for pose in [Pose::Normal, Pose::Charge, Pose::Ult] {
        let shapes: Vec<Shape> = data.shapes.iter().filter(|s| s.pose == pose || s.pose == Pose::Always).cloned().collect();
        pieces_by_pose.insert(pose, pieces(&shapes, Some(index), colliders));
    }

    let fixtures = pieces_by_pose[&Pose::Normal].iter().map(|piece| physics.add_fixture(&body, piece)).collect();
    set_mass(physics, &body, data.body.mass, data.poses[&Pose::Normal]);

    Ok(Player {
        slot,
        body,
        stats: export.stats.clone(),
        mass: data.body.mass,
        mass_by_pose: data.poses.clone(),
        pieces_by_pose,
        active_pose: Pose::Normal,
        fixtures,
        spawn,
        out_of_area: stage.out_of_area,
        hp: export.stats.hp,
        ult_gauge: 0.0,
        last_position: spawn,
        ground_wheel_count: 0,
        body_ground_count: 0,
        deck_ground_count: 0,
        is_charging_jump: false,
        charge_flag: false,
    })
}

/// 書き出しの形を、コライダーごとに番号を振ってフィクスチャの片に変える。
/// 同じ colliderIndex のものは同じコライダー（Unity の Collider2D）として扱う。
fn pieces(shapes: &[Shape], owner: Option<usize>, colliders: &mut Vec<Collider>) -> Vec<Piece> {
    let mut ids: HashMap<u32, ColliderId> = HashMap::new();
    let mut out = Vec::new();
    for shape in shapes {
        let id = *ids.entry(shape.collider_index).or_insert_with(|| {
            colliders.push(Collider { owner, tag: tag(&shape.tag), handler: handler(&shape.handlers) });
            ColliderId(colliders.len() as u32 - 1)
        });
        for geometry in geometries(shape) {
            out.push(Piece {
                collider: id,
                geometry,
                friction: shape.friction,
                restitution: shape.bounciness,
                sensor: shape.trigger,
            });
        }
    }
    out
}

fn geometries(shape: &Shape) -> Vec<Geometry> {
    match shape.kind {
        ShapeKind::Circle => vec![Geometry::Circle { center: shape.vertices[0], radius: shape.radius }],
        ShapeKind::Polygon => {
            if polygon_is_valid(&shape.vertices) {
                vec![Geometry::Polygon(shape.vertices.clone())]
            } else {
                vec![]
            }
        }
        // Box2D 2.4 にカプセルはないため、両端の円と間の長方形で近似する
        ShapeKind::Capsule => {
            let [a, b] = [shape.vertices[0], shape.vertices[1]];
            let r = shape.radius;
            let d = [b[0] - a[0], b[1] - a[1]];
            let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
            let mut out = vec![Geometry::Circle { center: a, radius: r }, Geometry::Circle { center: b, radius: r }];
            if len > LINEAR_SLOP {
                let n = [-d[1] / len * r, d[0] / len * r];
                let quad = vec![
                    [a[0] + n[0], a[1] + n[1]],
                    [a[0] - n[0], a[1] - n[1]],
                    [b[0] - n[0], b[1] - n[1]],
                    [b[0] + n[0], b[1] + n[1]],
                ];
                if polygon_is_valid(&quad) {
                    out.push(Geometry::Polygon(quad));
                }
            }
            out
        }
        // R2500 にはない。v2 でステージの仕掛けを入れるときに対応する
        ShapeKind::Edges => vec![],
    }
}

/// Unity の質量・重心・慣性モーメント（重心まわり）を Box2D に入れる（慣性モーメントは原点まわりに直す）
fn set_mass<P: Physics>(physics: &mut P, body: &P::Body, mass: f32, props: MassProps) {
    let c = props.center_of_mass;
    physics.set_mass(body, mass, c, props.inertia + mass * (c[0] * c[0] + c[1] * c[1]));
}

fn tag(name: &str) -> Tag {
    match name {
        "Ground" => Tag::Ground,
        "TireCollision" => Tag::Tire,
        "MainCollision" => Tag::Main,
        "AttackCollision" => Tag::Attack,
        _ => Tag::Other,
    }
}

fn handler(handlers: &[String]) -> Handler {
    for h in handlers {
        match h.as_str() {
            "WheelGroundCheck" => return Handler::WheelGroundCheck,
            "MainCollision" => return Handler::MainCollision,
            "AttackCollision" => return Handler::AttackCollision,
            _ => {}
        }
    }
    Handler::None
}

// ---- 小さな道具 ----

/// Unity の Vector3.normalized（長さが 1e-5 以下なら 0）
fn normalized(d: Vec2) -> Vec2 {
    let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
    if len > 1e-5 { [d[0] / len, d[1] / len] } else { [0.0, 0.0] }
}

/// Unity の Mathf.MoveTowards
fn move_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    if (target - current).abs() <= max_delta {
        target
    } else {
        current + (target - current).signum() * max_delta
    }
}

/// Unity の Mathf.DeltaAngle(0, a)：-180..180 に直す
fn delta_angle(a: f32) -> f32 {
    let d = a.rem_euclid(360.0);
    if d > 180.0 { d - 360.0 } else { d }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_angle_wraps() {
        assert_eq!(delta_angle(190.0), -170.0);
        assert_eq!(delta_angle(-190.0), 170.0);
        assert_eq!(delta_angle(180.0), 180.0);
    }

    #[test]
    fn move_towards_matches_unity() {
        assert_eq!(move_towards(1.0, 0.0, 0.3), 0.7);
        assert_eq!(move_towards(0.2, 0.0, 0.3), 0.0);
        assert_eq!(move_towards(-1.0, 0.0, 0.3), -0.7);
    }
}
