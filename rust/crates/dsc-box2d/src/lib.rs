//! C++ 版 Box2D 2.4.1 の最小限のバインディング（src/shim.cpp 経由）。
//! Unity の Physics2D に一番近い候補として、box2d-rs と照合結果を比べるために使う。

use std::ffi::c_void;
use std::ptr::NonNull;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContactEvent {
    pub a: u32,
    pub b: u32,
    pub begin: u32,
}

unsafe extern "C" {
    fn dscb_world_new(gx: f32, gy: f32) -> *mut c_void;
    fn dscb_world_free(sim: *mut c_void);
    fn dscb_world_step(sim: *mut c_void, dt: f32, velocity_iterations: i32, position_iterations: i32);
    fn dscb_world_contact_count(sim: *mut c_void) -> i32;
    fn dscb_events_len(sim: *mut c_void) -> usize;
    fn dscb_events_ptr(sim: *mut c_void) -> *const ContactEvent;
    fn dscb_events_clear(sim: *mut c_void);
    fn dscb_body_new(
        sim: *mut c_void,
        body_type: i32,
        x: f32,
        y: f32,
        angle: f32,
        bullet: i32,
        gravity_scale: f32,
        linear_damping: f32,
        angular_damping: f32,
    ) -> *mut c_void;
    fn dscb_add_circle(body: *mut c_void, cx: f32, cy: f32, radius: f32, friction: f32, restitution: f32, sensor: i32, user: u32) -> *mut c_void;
    fn dscb_add_polygon(body: *mut c_void, xy: *const f32, n: i32, friction: f32, restitution: f32, sensor: i32, user: u32) -> *mut c_void;
    fn dscb_destroy_fixture(body: *mut c_void, fixture: *mut c_void);
    fn dscb_set_mass(body: *mut c_void, mass: f32, cx: f32, cy: f32, inertia: f32);
    fn dscb_body_state(body: *const c_void, out: *mut f32);
    fn dscb_set_transform(body: *mut c_void, x: f32, y: f32, angle: f32);
    fn dscb_set_linear_velocity(body: *mut c_void, vx: f32, vy: f32);
    fn dscb_set_angular_velocity(body: *mut c_void, w: f32);
    fn dscb_apply_torque(body: *mut c_void, torque: f32, wake: i32);
}

/// World が持っている剛体。World より長く使ってはいけない（World の操作経由でだけ使う）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Body(NonNull<c_void>);

/// 剛体に付いたフィクスチャ。消した後に使ってはいけない
#[derive(Debug, PartialEq, Eq)]
pub struct Fixture(NonNull<c_void>);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyState {
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub vx: f32,
    pub vy: f32,
    pub w: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyDef {
    pub dynamic: bool,
    pub pos: [f32; 2],
    /// ラジアン
    pub angle: f32,
    pub bullet: bool,
    pub gravity_scale: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixtureDef {
    pub friction: f32,
    pub restitution: f32,
    pub sensor: bool,
    /// 接触イベントで返ってくる番号
    pub user: u32,
}

pub struct World(NonNull<c_void>);

impl World {
    pub fn new(gravity: [f32; 2]) -> Self {
        Self(NonNull::new(unsafe { dscb_world_new(gravity[0], gravity[1]) }).expect("b2World"))
    }

    pub fn step(&mut self, dt: f32, velocity_iterations: i32, position_iterations: i32) {
        unsafe { dscb_world_step(self.0.as_ptr(), dt, velocity_iterations, position_iterations) }
    }

    pub fn contact_count(&self) -> usize {
        unsafe { dscb_world_contact_count(self.0.as_ptr()) as usize }
    }

    /// 前回から今までの接触の開始・終了を取り出す
    pub fn drain_events(&mut self, out: &mut Vec<ContactEvent>) {
        unsafe {
            let len = dscb_events_len(self.0.as_ptr());
            if len > 0 {
                out.extend_from_slice(std::slice::from_raw_parts(dscb_events_ptr(self.0.as_ptr()), len));
                dscb_events_clear(self.0.as_ptr());
            }
        }
    }

    pub fn create_body(&mut self, def: &BodyDef) -> Body {
        let ptr = unsafe {
            dscb_body_new(
                self.0.as_ptr(),
                if def.dynamic { 2 } else { 0 },
                def.pos[0],
                def.pos[1],
                def.angle,
                def.bullet as i32,
                def.gravity_scale,
                def.linear_damping,
                def.angular_damping,
            )
        };
        Body(NonNull::new(ptr).expect("b2Body"))
    }

    pub fn add_circle(&mut self, body: Body, center: [f32; 2], radius: f32, def: FixtureDef) -> Fixture {
        let ptr = unsafe {
            dscb_add_circle(body.0.as_ptr(), center[0], center[1], radius, def.friction, def.restitution, def.sensor as i32, def.user)
        };
        Fixture(NonNull::new(ptr).expect("b2Fixture"))
    }

    /// 3〜8 点の凸多角形。潰れた多角形を渡すと Box2D が 2x2 の箱に置き換えるので、呼ぶ側で除いておく
    pub fn add_polygon(&mut self, body: Body, points: &[[f32; 2]], def: FixtureDef) -> Fixture {
        assert!((3..=8).contains(&points.len()), "polygon needs 3..=8 points");
        let xy: Vec<f32> = points.iter().flatten().copied().collect();
        let ptr = unsafe {
            dscb_add_polygon(body.0.as_ptr(), xy.as_ptr(), points.len() as i32, def.friction, def.restitution, def.sensor as i32, def.user)
        };
        Fixture(NonNull::new(ptr).expect("b2Fixture"))
    }

    /// 触れていた接触は、その場で終了イベントになる
    pub fn destroy_fixture(&mut self, body: Body, fixture: Fixture) {
        unsafe { dscb_destroy_fixture(body.0.as_ptr(), fixture.0.as_ptr()) }
    }

    /// inertia は原点まわり（b2MassData::I）
    pub fn set_mass(&mut self, body: Body, mass: f32, center: [f32; 2], inertia: f32) {
        unsafe { dscb_set_mass(body.0.as_ptr(), mass, center[0], center[1], inertia) }
    }

    pub fn state(&self, body: Body) -> BodyState {
        let mut out = [0.0f32; 6];
        unsafe { dscb_body_state(body.0.as_ptr(), out.as_mut_ptr()) };
        BodyState { x: out[0], y: out[1], angle: out[2], vx: out[3], vy: out[4], w: out[5] }
    }

    pub fn set_transform(&mut self, body: Body, pos: [f32; 2], angle: f32) {
        unsafe { dscb_set_transform(body.0.as_ptr(), pos[0], pos[1], angle) }
    }

    pub fn set_linear_velocity(&mut self, body: Body, v: [f32; 2]) {
        unsafe { dscb_set_linear_velocity(body.0.as_ptr(), v[0], v[1]) }
    }

    pub fn set_angular_velocity(&mut self, body: Body, w: f32) {
        unsafe { dscb_set_angular_velocity(body.0.as_ptr(), w) }
    }

    pub fn apply_torque(&mut self, body: Body, torque: f32) {
        unsafe { dscb_apply_torque(body.0.as_ptr(), torque, 1) }
    }
}

impl Drop for World {
    fn drop(&mut self) {
        unsafe { dscb_world_free(self.0.as_ptr()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falling_box_lands_and_reports_contacts() {
        let mut world = World::new([0.0, -9.81]);
        let body_def = |dynamic, pos| BodyDef {
            dynamic,
            pos,
            angle: 0.0,
            bullet: false,
            gravity_scale: 1.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
        };
        let ground = world.create_body(&body_def(false, [0.0, 0.0]));
        let def = |user| FixtureDef { friction: 0.5, restitution: 0.0, sensor: false, user };
        world.add_polygon(ground, &[[-5.0, -1.0], [5.0, -1.0], [5.0, 0.0], [-5.0, 0.0]], def(1));
        let body = world.create_body(&body_def(true, [0.0, 2.0]));
        world.add_circle(body, [0.0, 0.0], 0.5, def(2));
        world.set_mass(body, 1.0, [0.0, 0.0], 0.1);

        let mut events = Vec::new();
        for _ in 0..100 {
            world.step(0.02, 8, 3);
            world.drain_events(&mut events);
        }
        let s = world.state(body);
        assert!((s.y - 0.5).abs() < 0.02, "rests on the ground: {s:?}");
        assert_eq!(events.first().map(|e| e.begin), Some(1));
        assert_eq!(world.contact_count(), 1);
    }
}
