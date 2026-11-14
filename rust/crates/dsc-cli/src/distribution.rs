//! ランダム照合 R1：個々の軌道ではなく、キャラごとの分布を Unity と比べる（仕様書「照合」）。
//!
//! - 起き上がり成功率（入力なしの区間の後、最後のステップで両輪が接地しているか）の差は 5 ポイント以内
//! - 最高点・跳んだ距離の中央値は、差が 5% 以内か、ブートストラップ法で求めた差の 95% 信頼区間が 0 を含めば合格
//!   （跳んだ距離のようにばらつきの大きい量では、数千サンプルでも 5% を判定できる精度にならないため）
//! - 止まったときの姿勢（立っている／横倒し／逆さま）の割合は参考として出す

use std::collections::BTreeMap;

use dsc_scenario::Scenario;
use dsc_scenario::rng::Pcg32;
use dsc_scenario::trace::TraceRow;

const RATE_TOL: f32 = 0.05;
const MEDIAN_TOL: f32 = 0.05;
const BOOTSTRAP: usize = 2000;

/// 止まったときの姿勢。剛体の角度で分ける
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rest {
    /// |角度| < 45°
    Upright,
    /// 45° 以上 135° 以下
    Side,
    /// |角度| > 135°
    UpsideDown,
}

/// 1プレイヤーぶんの結果
#[derive(Debug, Clone, Copy)]
struct Sample {
    /// 最後のステップで両輪が地面に着いているか
    upright: bool,
    rest: Rest,
    /// 一番高い位置と一番低い位置の差
    height: f32,
    /// 最初に着地した後、初めて跳んで（5ステップ以上浮いて）着地するまでの水平距離
    flight: Option<f32>,
}

#[derive(Default)]
struct Group {
    scenarios: usize,
    strict_failures: usize,
    errors: Vec<String>,
    unity: Vec<Sample>,
    sim: Vec<Sample>,
}

pub enum Metric {
    Rate { name: &'static str, unity: f32, sim: f32 },
    Median { name: &'static str, unity: f32, sim: f32, n: (usize, usize), ci: (f32, f32) },
}

impl Metric {
    pub fn ok(&self) -> bool {
        match *self {
            Metric::Rate { unity, sim, .. } => (unity - sim).abs() <= RATE_TOL,
            Metric::Median { unity, sim, ci, .. } => (unity - sim).abs() <= MEDIAN_TOL * unity.abs() || (ci.0 <= 0.0 && 0.0 <= ci.1),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Metric::Rate { name, .. } | Metric::Median { name, .. } => name,
        }
    }

    pub fn values(&self) -> (f32, f32) {
        match *self {
            Metric::Rate { unity, sim, .. } | Metric::Median { unity, sim, .. } => (unity, sim),
        }
    }
}

pub struct DistResult {
    pub character: String,
    pub scenarios: usize,
    pub strict_failures: usize,
    pub errors: Vec<String>,
    pub metrics: Vec<Metric>,
    /// 立っている／横倒し／逆さまの割合（unity, sim）
    pub rest: [(f32, f32); 3],
}

impl DistResult {
    pub fn ok(&self) -> bool {
        self.errors.is_empty() && self.strict_failures == 0 && self.metrics.iter().all(Metric::ok)
    }
}

#[derive(Default)]
pub struct Distribution {
    groups: BTreeMap<String, Group>,
}

impl Distribution {
    /// strict_ok：最初の10ステップの厳密比較に通ったか
    pub fn add(&mut self, s: &Scenario, traces: Result<(Vec<TraceRow>, Vec<TraceRow>), String>, strict_ok: bool) {
        let g = self.groups.entry(s.p1.clone()).or_default();
        g.scenarios += 1;
        match traces {
            Ok((u, r)) => {
                if !strict_ok {
                    g.strict_failures += 1;
                }
                for slot in [1, 2] {
                    g.unity.push(sample(&u, slot));
                    g.sim.push(sample(&r, slot));
                }
            }
            Err(e) => g.errors.push(e),
        }
    }

    pub fn results(self) -> Vec<DistResult> {
        self.groups
            .into_iter()
            .enumerate()
            .map(|(i, (character, g))| {
                let mut rng = Pcg32::new(1, i as u64);
                let rate = |v: &[Sample], f: &dyn Fn(&Sample) -> bool| v.iter().filter(|s| f(s)).count() as f32 / v.len().max(1) as f32;
                let heights = |v: &[Sample]| v.iter().map(|s| s.height).collect::<Vec<_>>();
                let flights = |v: &[Sample]| v.iter().filter_map(|s| s.flight).collect::<Vec<_>>();
                let rest = [Rest::Upright, Rest::Side, Rest::UpsideDown]
                    .map(|k| (rate(&g.unity, &|s| s.rest == k), rate(&g.sim, &|s| s.rest == k)));
                DistResult {
                    metrics: vec![
                        Metric::Rate { name: "upright rate", unity: rate(&g.unity, &|s| s.upright), sim: rate(&g.sim, &|s| s.upright) },
                        median_metric("median height", heights(&g.unity), heights(&g.sim), &mut rng),
                        median_metric("median flight distance", flights(&g.unity), flights(&g.sim), &mut rng),
                    ],
                    rest,
                    character,
                    scenarios: g.scenarios,
                    strict_failures: g.strict_failures,
                    errors: g.errors,
                }
            })
            .collect()
    }
}

/// 中央値の差（sim − unity）と、そのブートストラップ 95% 信頼区間
fn median_metric(name: &'static str, unity: Vec<f32>, sim: Vec<f32>, rng: &mut Pcg32) -> Metric {
    let mut diffs: Vec<f32> = (0..BOOTSTRAP).map(|_| median(resample(&sim, rng)) - median(resample(&unity, rng))).collect();
    diffs.sort_by(f32::total_cmp);
    let ci = (diffs[BOOTSTRAP * 25 / 1000], diffs[BOOTSTRAP * 975 / 1000 - 1]);
    Metric::Median { name, n: (unity.len(), sim.len()), unity: median(unity), sim: median(sim), ci }
}

fn resample(v: &[f32], rng: &mut Pcg32) -> Vec<f32> {
    if v.is_empty() {
        return vec![];
    }
    (0..v.len()).map(|_| v[rng.below(v.len() as u32) as usize]).collect()
}

fn sample(rows: &[TraceRow], slot: u8) -> Sample {
    let rows: Vec<&TraceRow> = rows.iter().filter(|r| r.slot == slot).collect();
    let ys = rows.iter().map(|r| r.y);
    let height = ys.clone().fold(f32::MIN, f32::max) - ys.fold(f32::MAX, f32::min);
    let last = rows.last();
    let angle = last.map_or(0.0, |r| r.rot.abs());
    Sample {
        upright: last.is_some_and(|r| r.ground >= 2),
        rest: if angle < 45.0 { Rest::Upright } else if angle <= 135.0 { Rest::Side } else { Rest::UpsideDown },
        height,
        flight: flight(&rows),
    }
}

fn flight(rows: &[&TraceRow]) -> Option<f32> {
    let touchdown = rows.iter().position(|r| r.ground > 0)?;
    let takeoff = (touchdown + 1..rows.len()).find(|&i| rows[i].ground == 0 && rows[i - 1].ground > 0)?;
    let landing = (takeoff..rows.len()).find(|&i| rows[i].ground > 0)?;
    (landing - takeoff >= 5).then(|| (rows[landing].x - rows[takeoff - 1].x).abs())
}

fn median(mut v: Vec<f32>) -> f32 {
    if v.is_empty() {
        return f32::NAN;
    }
    v.sort_by(f32::total_cmp);
    let n = v.len();
    if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_of_even_and_odd() {
        assert_eq!(median(vec![3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median(vec![4.0, 1.0, 2.0, 3.0]), 2.5);
        assert!(median(vec![]).is_nan());
    }

    #[test]
    fn identical_samples_pass_and_shifted_samples_fail() {
        let mut rng = Pcg32::new(1, 1);
        let base: Vec<f32> = (0..500).map(|i| i as f32 / 500.0).collect();
        assert!(median_metric("x", base.clone(), base.clone(), &mut rng).ok());
        let shifted: Vec<f32> = base.iter().map(|x| x + 0.3).collect();
        assert!(!median_metric("x", base, shifted, &mut rng).ok());
    }
}
