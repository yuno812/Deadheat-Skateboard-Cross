//! ランダム照合 R1：個々の軌道ではなく、キャラごとの分布を Unity と比べる（仕様書「照合」）。
//! 起き上がり成功率の差は 5 ポイント以内、最高点・着地点の中央値の差は 5% 以内。

use std::collections::BTreeMap;

use dsc_scenario::Scenario;
use dsc_scenario::trace::TraceRow;

/// 1プレイヤーぶんの結果
#[derive(Debug, Clone, Copy)]
struct Sample {
    /// 最後のステップで両輪が地面に着いているか
    upright: bool,
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

pub struct Metric {
    pub name: &'static str,
    pub unity: f32,
    pub sim: f32,
    pub tolerance: f32,
}

impl Metric {
    pub fn ok(&self) -> bool {
        (self.unity - self.sim).abs() <= self.tolerance
    }
}

pub struct DistResult {
    pub character: String,
    pub scenarios: usize,
    pub strict_failures: usize,
    pub errors: Vec<String>,
    pub flights: (usize, usize),
    pub metrics: Vec<Metric>,
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
            .map(|(character, g)| {
                let rate = |v: &[Sample]| v.iter().filter(|s| s.upright).count() as f32 / v.len().max(1) as f32;
                let height = |v: &[Sample]| median(v.iter().map(|s| s.height).collect());
                let flights = |v: &[Sample]| v.iter().filter_map(|s| s.flight).collect::<Vec<_>>();
                let (fu, fr) = (flights(&g.unity), flights(&g.sim));
                let (hu, fmu) = (height(&g.unity), median(fu.clone()));
                DistResult {
                    character,
                    scenarios: g.scenarios,
                    strict_failures: g.strict_failures,
                    errors: g.errors,
                    flights: (fu.len(), fr.len()),
                    metrics: vec![
                        Metric { name: "upright rate", unity: rate(&g.unity), sim: rate(&g.sim), tolerance: 0.05 },
                        Metric { name: "median height", unity: hu, sim: height(&g.sim), tolerance: 0.05 * hu },
                        Metric { name: "median flight distance", unity: fmu, sim: median(fr), tolerance: 0.05 * fmu },
                    ],
                }
            })
            .collect()
    }
}

fn sample(rows: &[TraceRow], slot: u8) -> Sample {
    let rows: Vec<&TraceRow> = rows.iter().filter(|r| r.slot == slot).collect();
    let ys = rows.iter().map(|r| r.y);
    let height = ys.clone().fold(f32::MIN, f32::max) - ys.fold(f32::MAX, f32::min);
    Sample { upright: rows.last().is_some_and(|r| r.ground >= 2), height, flight: flight(&rows) }
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
}
