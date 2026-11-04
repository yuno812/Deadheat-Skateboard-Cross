//! 地形：ステージの当たり判定の辺から、真下の床の高さと傾きを求め、空中での着地を予測する（E3・E4）。
//! 辺の一覧は、シミュレーターでは書き出しデータから、ゲームでは試合開始時にシーンの当たり判定から渡す。

/// 着地の予測
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Landing {
    /// 今から着地までの秒数
    pub time_s: f32,
    /// 着地したときの体の中心
    pub x: f32,
    pub y: f32,
    /// 着地点の地面の法線が、上方向から反時計回りに何度傾いているか（板をこの角度にすれば車輪から着く）
    pub surface_angle_deg: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Terrain {
    edges: Vec<([f32; 2], [f32; 2])>,
}

impl Terrain {
    /// 当たり判定の多角形（凸分割後の片）の辺。内側の辺が混ざっていても、空中の点から真下を見る限り問題ない
    pub fn from_polygons<'a>(polygons: impl IntoIterator<Item = &'a [[f32; 2]]>) -> Self {
        let mut edges = Vec::new();
        for poly in polygons {
            for i in 0..poly.len() {
                let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
                if a[0] != b[0] {
                    edges.push((a, b));
                }
            }
        }
        Self { edges }
    }

    /// (x, y) から真下にある一番近い面の高さと、その面の傾き（度）
    pub fn floor_below(&self, x: f32, y: f32) -> Option<(f32, f32)> {
        let mut best: Option<(f32, f32)> = None;
        for &(a, b) in &self.edges {
            let (lo, hi) = if a[0] < b[0] { (a, b) } else { (b, a) };
            if x < lo[0] || x > hi[0] {
                continue;
            }
            let t = (x - lo[0]) / (hi[0] - lo[0]);
            let hy = lo[1] + t * (hi[1] - lo[1]);
            if hy <= y && best.is_none_or(|(by, _)| hy > by) {
                // 左から右へ向かう辺の傾き。法線（上向き）が上方向から反時計回りに回った角度と同じ
                let angle = (hi[1] - lo[1]).atan2(hi[0] - lo[0]).to_degrees();
                best = Some((hy, angle));
            }
        }
        best
    }

    /// 放物線で落下を追い、車輪の下端（体の中心から ride_height 下）が床に着く時刻と場所を返す。
    /// max_time_s までに着かなければ None
    pub fn predict_landing(&self, pos: [f32; 2], vel: [f32; 2], gravity_y: f32, ride_height: f32, max_time_s: f32) -> Option<Landing> {
        const DT: f32 = 0.02;
        let (mut p, mut v) = (pos, vel);
        let mut t = 0.0;
        while t < max_time_s {
            v[1] += gravity_y * DT;
            p = [p[0] + v[0] * DT, p[1] + v[1] * DT];
            t += DT;
            let (floor, angle) = self.floor_below(p[0], p[1])?;
            if p[1] - ride_height <= floor {
                return Some(Landing { time_s: t, x: p[0], y: p[1], surface_angle_deg: angle });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp() -> Terrain {
        // 平らな床（y = 0、x < 0）と、右上がり 45° の坂（x >= 0）
        let flat: [[f32; 2]; 4] = [[-10.0, -1.0], [0.0, -1.0], [0.0, 0.0], [-10.0, 0.0]];
        let slope: [[f32; 2]; 3] = [[0.0, 0.0], [10.0, -1.0], [10.0, 10.0]];
        Terrain::from_polygons([&flat[..], &slope[..]])
    }

    #[test]
    fn floor_and_angle() {
        let t = ramp();
        let (y, a) = t.floor_below(-5.0, 3.0).unwrap();
        assert!((y - 0.0).abs() < 1e-5 && a.abs() < 1e-3);
        let (y, a) = t.floor_below(2.0, 5.0).unwrap();
        assert!((y - 2.0).abs() < 1e-4 && (a - 45.0).abs() < 1e-3, "{y} {a}");
        assert!(t.floor_below(-5.0, -0.5).is_some_and(|(y, _)| y < -0.5));
    }

    #[test]
    fn predicts_landing_on_the_slope() {
        let l = ramp().predict_landing([-1.0, 5.0], [3.0, 0.0], -9.81, 1.0, 3.0).unwrap();
        assert!(l.x > 0.0 && (l.surface_angle_deg - 45.0).abs() < 1e-3, "{l:?}");
        assert!(l.time_s > 0.5 && l.time_s < 1.0, "{l:?}");
    }
}
