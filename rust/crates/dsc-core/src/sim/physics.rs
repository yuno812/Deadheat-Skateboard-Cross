//! 物理エンジンとの境界。Match（MovePlayer の移植）はこのトレイト経由でだけエンジンを触る。
//! 実装は box2d-rs（rs.rs）と C++ 版 Box2D 2.4.1（cpp.rs）の2つ。角度・角速度はラジアン。

use crate::export::Vec2;

/// Unity のコライダー（凸分割前の1つの当たり判定）の番号。フィクスチャに持たせ、接触イベントで返ってくる
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ColliderId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodySpec {
    pub dynamic: bool,
    pub pos: Vec2,
    pub angle: f32,
    pub bullet: bool,
    pub gravity_scale: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyState {
    pub pos: Vec2,
    pub angle: f32,
    pub vel: Vec2,
    pub w: f32,
}

/// フィクスチャ1つぶんの形（コライダーの凸分割後の1片）
#[derive(Debug, Clone)]
pub struct Piece {
    pub collider: ColliderId,
    pub geometry: Geometry,
    pub friction: f32,
    pub restitution: f32,
    pub sensor: bool,
}

#[derive(Debug, Clone)]
pub enum Geometry {
    Circle { center: Vec2, radius: f32 },
    /// 3〜8 点の、潰れていない凸多角形（`polygon_is_valid` を通ったもの）
    Polygon(Vec<Vec2>),
}

pub trait Physics {
    type Body: Clone;
    type Fixture;

    fn new(gravity: Vec2) -> Self;
    fn create_body(&mut self, spec: &BodySpec) -> Self::Body;
    fn add_fixture(&mut self, body: &Self::Body, piece: &Piece) -> Self::Fixture;
    /// 触れていた接触は、その場で終了イベントになる
    fn destroy_fixture(&mut self, body: &Self::Body, fixture: Self::Fixture);
    /// inertia は原点まわり（Box2D の b2MassData::I）
    fn set_mass(&mut self, body: &Self::Body, mass: f32, center: Vec2, inertia: f32);
    fn state(&self, body: &Self::Body) -> BodyState;
    fn set_transform(&mut self, body: &Self::Body, pos: Vec2, angle: f32);
    fn set_linear_velocity(&mut self, body: &Self::Body, v: Vec2);
    fn set_angular_velocity(&mut self, body: &Self::Body, w: f32);
    fn apply_torque(&mut self, body: &Self::Body, torque: f32);
    fn step(&mut self, dt: f32, velocity_iterations: i32, position_iterations: i32);
    /// 前回から今までの接触の開始（true）・終了（false）を、フィクスチャの組ごとに取り出す
    fn drain_contacts(&mut self, out: &mut Vec<(ColliderId, ColliderId, bool)>);
    /// AABB が重なっているフィクスチャの組の数（速度の調査用）
    fn contact_count(&self) -> usize;
}

/// Box2D の b2PolygonShape::Set が受け付けるか（近すぎる頂点をまとめて3〜8点、面積が0でない）。
/// 受け付けない多角形を渡すと、Box2D は 2x2 の箱に置き換えてしまう
pub fn polygon_is_valid(vertices: &[Vec2]) -> bool {
    const LINEAR_SLOP: f32 = 0.005;
    let weld = 0.5 * LINEAR_SLOP;
    let mut unique: Vec<Vec2> = Vec::new();
    for &p in vertices {
        if unique.iter().all(|q| (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) >= weld * weld) {
            unique.push(p);
        }
    }
    if unique.len() < 3 || unique.len() > 8 {
        return false;
    }
    let n = unique.len();
    let area: f32 = (0..n)
        .map(|i| {
            let (p, q) = (unique[i], unique[(i + 1) % n]);
            p[0] * q[1] - q[0] * p[1]
        })
        .sum::<f32>()
        * 0.5;
    area.abs() > 4.0 * f32::EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn degenerate_polygons_are_rejected() {
        assert!(polygon_is_valid(&[[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]));
        assert!(!polygon_is_valid(&[[0.0, 0.0], [1.0, 0.0], [2.0, 0.0]]));
        assert!(!polygon_is_valid(&[[0.0, 0.0], [0.001, 0.0], [0.0, 1.0]]));
    }
}
