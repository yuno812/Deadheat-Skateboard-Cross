//! C++ 版 Box2D 2.4.1（dsc-box2d）による Physics の実装。

use dsc_box2d::{Body, BodyDef, ContactEvent, Fixture, FixtureDef, World};

use super::physics::{BodySpec, BodyState, ColliderId, Geometry, Physics, Piece};
use crate::export::Vec2;

pub struct CppPhysics {
    world: World,
    events: Vec<ContactEvent>,
}

impl Physics for CppPhysics {
    type Body = Body;
    type Fixture = Fixture;

    fn new(gravity: Vec2) -> Self {
        Self { world: World::new(gravity), events: Vec::new() }
    }

    fn create_body(&mut self, spec: &BodySpec) -> Self::Body {
        self.world.create_body(&BodyDef {
            dynamic: spec.dynamic,
            pos: spec.pos,
            angle: spec.angle,
            bullet: spec.bullet,
            gravity_scale: spec.gravity_scale,
            linear_damping: spec.linear_damping,
            angular_damping: spec.angular_damping,
        })
    }

    fn add_fixture(&mut self, body: &Self::Body, piece: &Piece) -> Self::Fixture {
        let def = FixtureDef {
            friction: piece.friction,
            restitution: piece.restitution,
            sensor: piece.sensor,
            user: piece.collider.0,
        };
        match &piece.geometry {
            Geometry::Circle { center, radius } => self.world.add_circle(*body, *center, *radius, def),
            Geometry::Polygon(points) => self.world.add_polygon(*body, points, def),
        }
    }

    fn destroy_fixture(&mut self, body: &Self::Body, fixture: Self::Fixture) {
        self.world.destroy_fixture(*body, fixture);
    }

    fn set_mass(&mut self, body: &Self::Body, mass: f32, center: Vec2, inertia: f32) {
        self.world.set_mass(*body, mass, center, inertia);
    }

    fn state(&self, body: &Self::Body) -> BodyState {
        let s = self.world.state(*body);
        BodyState { pos: [s.x, s.y], angle: s.angle, vel: [s.vx, s.vy], w: s.w }
    }

    fn set_transform(&mut self, body: &Self::Body, pos: Vec2, angle: f32) {
        self.world.set_transform(*body, pos, angle);
    }

    fn set_linear_velocity(&mut self, body: &Self::Body, v: Vec2) {
        self.world.set_linear_velocity(*body, v);
    }

    fn set_angular_velocity(&mut self, body: &Self::Body, w: f32) {
        self.world.set_angular_velocity(*body, w);
    }

    fn apply_torque(&mut self, body: &Self::Body, torque: f32) {
        self.world.apply_torque(*body, torque);
    }

    fn step(&mut self, dt: f32, velocity_iterations: i32, position_iterations: i32) {
        self.world.step(dt, velocity_iterations, position_iterations);
    }

    fn drain_contacts(&mut self, out: &mut Vec<(ColliderId, ColliderId, bool)>) {
        self.world.drain_events(&mut self.events);
        out.extend(self.events.drain(..).map(|e| (ColliderId(e.a), ColliderId(e.b), e.begin != 0)));
    }

    fn contact_count(&self) -> usize {
        self.world.contact_count()
    }
}
