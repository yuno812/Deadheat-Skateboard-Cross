// Box2D 2.4.1（C++）を Rust から呼ぶための薄い仲介層。必要な操作だけを extern "C" で出す。
// 接触の開始・終了は、フィクスチャの userData（コライダー番号）の組として記録し、Rust 側がまとめて取り出す。

#include <cstdint>
#include <vector>

#include "box2d/box2d.h"

namespace {

struct Event {
    uint32_t a;
    uint32_t b;
    uint32_t begin;
};

struct Listener : b2ContactListener {
    std::vector<Event> events;

    void BeginContact(b2Contact* c) override { Push(c, 1); }
    void EndContact(b2Contact* c) override { Push(c, 0); }

    void Push(b2Contact* c, uint32_t begin) {
        events.push_back({
            static_cast<uint32_t>(c->GetFixtureA()->GetUserData().pointer),
            static_cast<uint32_t>(c->GetFixtureB()->GetUserData().pointer),
            begin,
        });
    }
};

struct Sim {
    b2World world;
    Listener listener;

    explicit Sim(b2Vec2 gravity) : world(gravity) { world.SetContactListener(&listener); }
};

b2Fixture* AddFixture(b2Body* body, const b2Shape* shape, float friction, float restitution, int sensor, uint32_t user) {
    b2FixtureDef def;
    def.shape = shape;
    def.friction = friction;
    def.restitution = restitution;
    def.density = 0.0f;  // 質量は dscb_set_mass で入れる
    def.isSensor = sensor != 0;
    def.userData.pointer = user;
    return body->CreateFixture(&def);
}

}  // namespace

extern "C" {

Sim* dscb_world_new(float gx, float gy) { return new Sim(b2Vec2(gx, gy)); }

void dscb_world_free(Sim* sim) { delete sim; }

void dscb_world_step(Sim* sim, float dt, int velocity_iterations, int position_iterations) {
    sim->world.Step(dt, velocity_iterations, position_iterations);
}

int dscb_world_contact_count(Sim* sim) { return sim->world.GetContactCount(); }

size_t dscb_events_len(Sim* sim) { return sim->listener.events.size(); }

const Event* dscb_events_ptr(Sim* sim) { return sim->listener.events.data(); }

void dscb_events_clear(Sim* sim) { sim->listener.events.clear(); }

// body_type: 0 = static, 2 = dynamic（b2BodyType と同じ値）
b2Body* dscb_body_new(Sim* sim, int body_type, float x, float y, float angle, int bullet, float gravity_scale,
                      float linear_damping, float angular_damping) {
    b2BodyDef def;
    def.type = static_cast<b2BodyType>(body_type);
    def.position.Set(x, y);
    def.angle = angle;
    def.bullet = bullet != 0;
    def.gravityScale = gravity_scale;
    def.linearDamping = linear_damping;
    def.angularDamping = angular_damping;
    return sim->world.CreateBody(&def);
}

b2Fixture* dscb_add_circle(b2Body* body, float cx, float cy, float radius, float friction, float restitution,
                           int sensor, uint32_t user) {
    b2CircleShape shape;
    shape.m_p.Set(cx, cy);
    shape.m_radius = radius;
    return AddFixture(body, &shape, friction, restitution, sensor, user);
}

// xy：x0, y0, x1, y1, ... の順に n 点（3 <= n <= 8、潰れていないこと）
b2Fixture* dscb_add_polygon(b2Body* body, const float* xy, int n, float friction, float restitution, int sensor,
                            uint32_t user) {
    b2Vec2 points[b2_maxPolygonVertices];
    for (int i = 0; i < n; ++i) points[i].Set(xy[2 * i], xy[2 * i + 1]);
    b2PolygonShape shape;
    shape.Set(points, n);
    return AddFixture(body, &shape, friction, restitution, sensor, user);
}

void dscb_destroy_fixture(b2Body* body, b2Fixture* fixture) { body->DestroyFixture(fixture); }

// inertia は原点まわり（b2MassData::I と同じ）
void dscb_set_mass(b2Body* body, float mass, float cx, float cy, float inertia) {
    b2MassData data;
    data.mass = mass;
    data.center.Set(cx, cy);
    data.I = inertia;
    body->SetMassData(&data);
}

// out：x, y, angle, vx, vy, w（ラジアン）
void dscb_body_state(const b2Body* body, float* out) {
    const b2Vec2& p = body->GetPosition();
    const b2Vec2& v = body->GetLinearVelocity();
    out[0] = p.x;
    out[1] = p.y;
    out[2] = body->GetAngle();
    out[3] = v.x;
    out[4] = v.y;
    out[5] = body->GetAngularVelocity();
}

void dscb_set_transform(b2Body* body, float x, float y, float angle) { body->SetTransform(b2Vec2(x, y), angle); }

void dscb_set_linear_velocity(b2Body* body, float vx, float vy) { body->SetLinearVelocity(b2Vec2(vx, vy)); }

void dscb_set_angular_velocity(b2Body* body, float w) { body->SetAngularVelocity(w); }

void dscb_apply_torque(b2Body* body, float torque, int wake) { body->ApplyTorque(torque, wake != 0); }

}  // extern "C"
