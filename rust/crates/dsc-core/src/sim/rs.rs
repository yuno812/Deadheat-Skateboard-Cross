//! box2d-rs（Box2D 2.4.1 の Rust 移植）による Physics の実装。

use std::cell::RefCell;
use std::rc::Rc;

use box2d_rs::b2_body::{B2body, B2bodyDef, B2bodyType, BodyPtr};
use box2d_rs::b2_contact::B2contactDynTrait;
use box2d_rs::b2_fixture::{B2fixtureDef, FixturePtr};
use box2d_rs::b2_math::B2vec2;
use box2d_rs::b2_shape::{B2massData, ShapeDefPtr};
use box2d_rs::b2_world::{B2world, B2worldPtr};
use box2d_rs::b2_world_callbacks::B2contactListener;
use box2d_rs::b2rs_common::UserDataType;
use box2d_rs::shapes::b2_circle_shape::B2circleShape;
use box2d_rs::shapes::b2_polygon_shape::B2polygonShape;

use super::physics::{BodySpec, BodyState, ColliderId, Geometry, Physics, Piece};
use crate::export::Vec2;

#[derive(Default, Clone, Debug)]
pub struct Ud;

impl UserDataType for Ud {
    type Fixture = ColliderId;
    type Body = ();
    type Joint = ();
}

type Events = Rc<RefCell<Vec<(ColliderId, ColliderId, bool)>>>;

/// begin/end_contact をためておく（ステップ中は World が借用されているため、取り出しは後で行う）
struct Listener(Events);

impl B2contactListener<Ud> for Listener {
    fn begin_contact(&mut self, contact: &mut dyn B2contactDynTrait<Ud>) {
        self.record(contact, true);
    }

    fn end_contact(&mut self, contact: &mut dyn B2contactDynTrait<Ud>) {
        self.record(contact, false);
    }
}

impl Listener {
    fn record(&mut self, contact: &mut dyn B2contactDynTrait<Ud>, begin: bool) {
        let base = contact.get_base();
        let a = base.get_fixture_a().borrow().get_user_data().unwrap_or_default();
        let b = base.get_fixture_b().borrow().get_user_data().unwrap_or_default();
        self.0.borrow_mut().push((a, b, begin));
    }
}

pub struct RsPhysics {
    world: B2worldPtr<Ud>,
    events: Events,
}

impl Physics for RsPhysics {
    type Body = BodyPtr<Ud>;
    type Fixture = FixturePtr<Ud>;

    fn new(gravity: Vec2) -> Self {
        let world = B2world::<Ud>::new(v(gravity));
        let events = Events::default();
        world.borrow_mut().set_contact_listener(Rc::new(RefCell::new(Listener(events.clone()))));
        Self { world, events }
    }

    fn create_body(&mut self, spec: &BodySpec) -> Self::Body {
        let def = B2bodyDef::<Ud> {
            body_type: if spec.dynamic { B2bodyType::B2DynamicBody } else { B2bodyType::B2StaticBody },
            position: v(spec.pos),
            angle: spec.angle,
            bullet: spec.bullet,
            gravity_scale: spec.gravity_scale,
            linear_damping: spec.linear_damping,
            angular_damping: spec.angular_damping,
            ..Default::default()
        };
        B2world::create_body(self.world.clone(), &def)
    }

    fn add_fixture(&mut self, body: &Self::Body, piece: &Piece) -> Self::Fixture {
        let shape: ShapeDefPtr = match &piece.geometry {
            Geometry::Circle { center, radius } => {
                let mut c = B2circleShape::default();
                c.base.m_radius = *radius;
                c.m_p = v(*center);
                Rc::new(RefCell::new(c))
            }
            Geometry::Polygon(points) => {
                let mut p = B2polygonShape::default();
                let points: Vec<B2vec2> = points.iter().map(|&q| v(q)).collect();
                p.set(&points);
                Rc::new(RefCell::new(p))
            }
        };
        let def = B2fixtureDef::<Ud> {
            shape: Some(shape),
            user_data: Some(piece.collider),
            friction: piece.friction,
            restitution: piece.restitution,
            density: 0.0,
            is_sensor: piece.sensor,
            ..Default::default()
        };
        B2body::create_fixture(body.clone(), &def)
    }

    fn destroy_fixture(&mut self, body: &Self::Body, fixture: Self::Fixture) {
        B2body::destroy_fixture(body.clone(), fixture);
    }

    fn set_mass(&mut self, body: &Self::Body, mass: f32, center: Vec2, inertia: f32) {
        body.borrow_mut().set_mass_data(&B2massData { mass, center: v(center), i: inertia });
    }

    fn state(&self, body: &Self::Body) -> BodyState {
        let b = body.borrow();
        let (p, vel) = (b.get_position(), b.get_linear_velocity());
        BodyState { pos: [p.x, p.y], angle: b.get_angle(), vel: [vel.x, vel.y], w: b.get_angular_velocity() }
    }

    fn set_transform(&mut self, body: &Self::Body, pos: Vec2, angle: f32) {
        body.borrow_mut().set_transform(v(pos), angle);
    }

    fn set_linear_velocity(&mut self, body: &Self::Body, vel: Vec2) {
        body.borrow_mut().set_linear_velocity(v(vel));
    }

    fn set_angular_velocity(&mut self, body: &Self::Body, w: f32) {
        body.borrow_mut().set_angular_velocity(w);
    }

    fn apply_torque(&mut self, body: &Self::Body, torque: f32) {
        body.borrow_mut().apply_torque(torque, true);
    }

    fn step(&mut self, dt: f32, velocity_iterations: i32, position_iterations: i32) {
        self.world.borrow_mut().step(dt, velocity_iterations, position_iterations);
    }

    fn drain_contacts(&mut self, out: &mut Vec<(ColliderId, ColliderId, bool)>) {
        out.append(&mut self.events.borrow_mut());
    }

    fn contact_count(&self) -> usize {
        self.world.borrow().get_contact_count()
    }
}

fn v(p: Vec2) -> B2vec2 {
    B2vec2::new(p[0], p[1])
}
