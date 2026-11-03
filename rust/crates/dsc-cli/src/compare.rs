//! `dsc compare`：Unity とシミュレーターのトレースを照合し、
//! data/results/compare-<STAGE>-<ENGINE>.md にレポートを書く（仕様書「照合」）。
//!
//! - 最初の10ステップは厳密に比べる（位置の差 0.001 未満、角度の差 0.1° 未満）
//! - それ以降はシナリオごとの指標で比べる（S1〜S11、仕様書の照合表）
//! - ランダム照合 R1 は、キャラごとの分布で比べる（distribution.rs）

use std::fmt::Write as _;
use std::path::Path;

use dsc_scenario::Scenario;
use dsc_scenario::trace::{self, TraceRow};

use crate::Options;
use crate::distribution::{DistResult, Distribution};
use crate::run::load_scenarios;

const STRICT_STEPS: u32 = 10;
const POS_TOL: f32 = 0.001;
const ANGLE_TOL: f32 = 0.1;

struct Check {
    name: &'static str,
    unity: f32,
    sim: f32,
    tolerance: f32,
}

impl Check {
    /// NaN は「起きなかった」（例：900度/秒に届かなかった）。両方とも起きなければ一致とみなす
    fn ok(&self) -> bool {
        (self.unity.is_nan() && self.sim.is_nan()) || (self.unity - self.sim).abs() <= self.tolerance
    }
}

struct Result_ {
    id: String,
    first_divergence: Option<(u32, u8)>,
    checks: Vec<Check>,
    error: Option<String>,
}

impl Result_ {
    fn strict_ok(&self) -> bool {
        self.first_divergence.is_none_or(|(step, _)| step >= STRICT_STEPS)
    }

    fn ok(&self) -> bool {
        self.error.is_none() && self.strict_ok() && self.checks.iter().all(Check::ok)
    }
}

pub fn compare(data_dir: &Path, opts: &Options) -> Result<bool, Box<dyn std::error::Error>> {
    let scenarios = load_scenarios(data_dir, opts)?;
    let unity_dir = data_dir.join("traces").join("unity");
    let sim_dir = opts.engine.trace_dir(data_dir);
    let mut results = Vec::new();
    let mut distribution = Distribution::default();
    for s in &scenarios {
        let load = |dir: &Path| -> Result<Vec<TraceRow>, String> {
            let path = dir.join(format!("{}.csv", s.id));
            let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            trace::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
        };
        let traces = match (load(&unity_dir), load(&sim_dir)) {
            (Ok(u), Ok(r)) if u.len() == r.len() => Ok((u, r)),
            (Ok(u), Ok(r)) => Err(format!("{}: rows differ: unity {} / sim {}", s.id, u.len(), r.len())),
            (Err(e), _) | (_, Err(e)) => Err(e),
        };
        if s.id.starts_with("R1-") {
            let strict_ok = traces.as_ref().is_ok_and(|(u, r)| first_divergence(u, r).is_none_or(|(step, _)| step >= STRICT_STEPS));
            distribution.add(s, traces, strict_ok);
            continue;
        }
        results.push(match traces {
            Ok((u, r)) => compare_one(s, &u, &r),
            Err(e) => Result_ { id: s.id.clone(), first_divergence: None, checks: vec![], error: Some(e) },
        });
    }
    let dist = distribution.results();

    let mut report = render(&format!("{} / {}", opts.stage, opts.engine.name()), &results);
    if !dist.is_empty() {
        report.push_str(&render_distribution(&dist));
    }
    let out = data_dir.join("results");
    std::fs::create_dir_all(&out)?;
    let path = out.join(format!("compare-{}-{}.md", opts.stage, opts.engine.name()));
    std::fs::write(&path, &report)?;

    let passed = results.iter().filter(|r| r.ok()).count();
    for r in results.iter().filter(|r| !r.ok()) {
        println!("FAIL {}", summary_line(r));
    }
    let dist_passed = dist.iter().filter(|d| d.ok()).count();
    for d in dist.iter().filter(|d| !d.ok()) {
        println!("FAIL R1-{}: {}", d.character, dist_summary(d));
    }
    if !results.is_empty() {
        println!("{passed}/{} scenarios passed.", results.len());
    }
    if !dist.is_empty() {
        println!("R1: {dist_passed}/{} characters passed.", dist.len());
    }
    println!("report: {}", path.display());
    Ok(passed == results.len() && dist_passed == dist.len())
}

fn first_divergence(unity: &[TraceRow], sim: &[TraceRow]) -> Option<(u32, u8)> {
    unity.iter().zip(sim).find_map(|(u, r)| {
        let dpos = ((u.x - r.x).powi(2) + (u.y - r.y).powi(2)).sqrt();
        let dangle = angle_diff(u.rot, r.rot);
        (dpos >= POS_TOL || dangle >= ANGLE_TOL).then_some((u.step, u.slot))
    })
}

fn dist_summary(d: &DistResult) -> String {
    let mut parts: Vec<String> = d
        .metrics
        .iter()
        .filter(|m| !m.ok())
        .map(|m| format!("{} unity {:.3} sim {:.3}", m.name, m.unity, m.sim))
        .collect();
    if d.strict_failures > 0 {
        parts.push(format!("{} scenarios diverge within {STRICT_STEPS} steps", d.strict_failures));
    }
    parts.extend(d.errors.iter().take(3).cloned());
    parts.join("; ")
}

fn render_distribution(dist: &[DistResult]) -> String {
    let mut md = String::new();
    let passed = dist.iter().filter(|d| d.ok()).count();
    let _ = writeln!(md, "
## R1（ランダム照合）
");
    let _ = writeln!(md, "{passed}/{} キャラ合格。起き上がり成功率の差 5 ポイント以内、最高点・跳んだ距離の中央値の差 5% 以内。
", dist.len());
    let _ = writeln!(md, "| キャラ | 合否 | シナリオ | 10ステップ以内のずれ | 起き上がり率 unity / sim | 最高点の中央値 unity / sim | 跳んだ距離の中央値 unity / sim（サンプル数） |");
    let _ = writeln!(md, "| --- | --- | --- | --- | --- | --- | --- |");
    for d in dist {
        let m = |i: usize| {
            let x = &d.metrics[i];
            format!("{}{:.3} / {:.3}", if x.ok() { "" } else { "**✗** " }, x.unity, x.sim)
        };
        let _ = writeln!(
            md,
            "| {} | {} | {} | {} | {} | {} | {}（{} / {}） |",
            d.character,
            if d.ok() { "○" } else { "✗" },
            d.scenarios,
            d.strict_failures,
            m(0),
            m(1),
            m(2),
            d.flights.0,
            d.flights.1
        );
    }
    md
}

fn compare_one(s: &Scenario, unity: &[TraceRow], sim: &[TraceRow]) -> Result_ {
    let mut result = Result_ { id: s.id.clone(), first_divergence: None, checks: vec![], error: None };
    if unity.len() != sim.len() {
        result.error = Some(format!("rows differ: unity {} / sim {}", unity.len(), sim.len()));
        return result;
    }
    result.first_divergence = first_divergence(unity, sim);

    let kind = s.id.split('-').next().unwrap_or_default();
    if kind == "S9" {
        stomp(unity, sim, &mut result.checks);
    }
    for slot in [1u8, 2] {
        let u: Vec<&TraceRow> = unity.iter().filter(|r| r.slot == slot).collect();
        let r: Vec<&TraceRow> = sim.iter().filter(|r| r.slot == slot).collect();
        match kind {
            "S1" => s1(&u, &r, &mut result.checks),
            "S2" => s2(s, slot, &u, &r, &mut result.checks),
            "S3" | "S4" => jump(kind, &u, &r, &mut result.checks),
            "S5" => s5(&u, &r, &mut result.checks),
            "S6" | "S7" => recovery(s, slot, &u, &r, &mut result.checks),
            "S8" => hits(&u, &r, &mut result.checks),
            "S10" => hp_loss(&u, &r, 2.0, &mut result.checks),
            "S11" => ground_transitions(&u, &r, &mut result.checks),
            _ => {}
        }
    }
    result
}

/// S1：静止位置 ±0.02、接地カウントが一致
fn s1(u: &[&TraceRow], r: &[&TraceRow], checks: &mut Vec<Check>) {
    let (lu, lr) = (u[u.len() - 1], r[r.len() - 1]);
    checks.push(Check { name: "rest x", unity: lu.x, sim: lr.x, tolerance: 0.02 });
    checks.push(Check { name: "rest y", unity: lu.y, sim: lr.y, tolerance: 0.02 });
    checks.push(Check { name: "rest ground", unity: lu.ground as f32, sim: lr.ground as f32, tolerance: 0.0 });
}

/// S2：最高速度に届くまでの時間 ±3%（Unityの最大 |vx| の 90% に初めて届いたステップ）と最大 |vx| ±3%
fn s2(s: &Scenario, slot: u8, u: &[&TraceRow], r: &[&TraceRow], checks: &mut Vec<Check>) {
    let from = s.inputs.iter().filter(|i| i.slot == slot).map(|i| i.from).min().unwrap_or(0);
    let vmax = |rows: &[&TraceRow]| rows.iter().map(|x| x.vx.abs()).fold(0.0f32, f32::max);
    let (vu, vr) = (vmax(u), vmax(r));
    let reach = |rows: &[&TraceRow]| {
        rows.iter().find(|x| x.vx.abs() >= 0.9 * vu).map_or(f32::NAN, |x| (x.step - from) as f32)
    };
    let (tu, tr) = (reach(u), reach(r));
    checks.push(Check { name: "steps to 90% vmax", unity: tu, sim: tr, tolerance: (0.03 * tu).max(1.0) });
    checks.push(Check { name: "max |vx|", unity: vu, sim: vr, tolerance: 0.03 * vu });
}

/// S3：最高点の高さ ±3%、滞空時間 ±1ステップ。S4：着地点のx ±5%（離した位置からの移動量に対して）
fn jump(kind: &str, u: &[&TraceRow], r: &[&TraceRow], checks: &mut Vec<Check>) {
    let (Some(ju), Some(jr)) = (jump_stats(u), jump_stats(r)) else {
        checks.push(Check { name: "jumped", unity: jump_stats(u).is_some() as u8 as f32, sim: jump_stats(r).is_some() as u8 as f32, tolerance: 0.0 });
        return;
    };
    if kind == "S3" {
        checks.push(Check { name: "apex height", unity: ju.height, sim: jr.height, tolerance: 0.03 * ju.height.abs() });
        checks.push(Check { name: "airtime steps", unity: ju.airtime, sim: jr.airtime, tolerance: 1.0 });
    } else {
        let tolerance = (0.05 * (ju.land_x - ju.release_x).abs()).max(0.02);
        checks.push(Check { name: "landing x", unity: ju.land_x, sim: jr.land_x, tolerance });
    }
}

struct JumpStats {
    release_x: f32,
    height: f32,
    airtime: f32,
    land_x: f32,
}

/// 上向きの速さが急に増えたステップをジャンプとみなし、そこからの最高点と再接地を調べる
fn jump_stats(rows: &[&TraceRow]) -> Option<JumpStats> {
    let i = (1..rows.len()).find(|&i| rows[i].vy - rows[i - 1].vy > 3.0)?;
    let base = rows[i - 1];
    let apex = rows[i..].iter().map(|x| x.y).fold(f32::MIN, f32::max);
    let land = (i + 2..rows.len()).find(|&k| rows[k].ground > 0 && rows[k - 1].ground == 0)?;
    Some(JumpStats {
        release_x: base.x,
        height: apex - base.y,
        airtime: (land - i) as f32,
        land_x: rows[land].x,
    })
}

/// S5：角加速度 ±2%（1ステップ目の角速度から）、900度/秒の制限に届くステップ ±1
fn s5(u: &[&TraceRow], r: &[&TraceRow], checks: &mut Vec<Check>) {
    checks.push(Check { name: "first-step w", unity: u[0].w, sim: r[0].w, tolerance: 0.02 * u[0].w.abs() });
    let limit = |rows: &[&TraceRow]| rows.iter().find(|x| x.w.abs() >= 899.0).map_or(f32::NAN, |x| x.step as f32);
    checks.push(Check { name: "step w reaches 900", unity: limit(u), sim: limit(r), tolerance: 1.0 });
}

/// S6・S7：起き上がれたか（回し始めてから、両輪が地面に着いたか）が一致、起き上がるまでの時間 ±10%
fn recovery(s: &Scenario, slot: u8, u: &[&TraceRow], r: &[&TraceRow], checks: &mut Vec<Check>) {
    let from = s.inputs.iter().filter(|i| i.slot == slot).map(|i| i.from).min().unwrap_or(0);
    let time = |rows: &[&TraceRow]| {
        rows.iter().find(|x| x.step >= from && x.ground >= 2).map_or(f32::NAN, |x| (x.step - from) as f32)
    };
    let (tu, tr) = (time(u), time(r));
    checks.push(Check { name: "recovered", unity: (!tu.is_nan()) as u8 as f32, sim: (!tr.is_nan()) as u8 as f32, tolerance: 0.0 });
    if !tu.is_nan() && !tr.is_nan() {
        checks.push(Check { name: "steps to recover", unity: tu, sim: tr, tolerance: (0.1 * tu).max(2.0) });
    }
}

/// S8：接触の種類（HitSame は速さ10、HitAttack は速さ25 に置き換わる）と発生ステップ ±1、そのときのHPの変化が一致。
/// 弾かれた後の動きはカオス的に分かれるので、比べるのは最初の接触だけ
fn hits(u: &[&TraceRow], r: &[&TraceRow], checks: &mut Vec<Check>) {
    let first_hit = |rows: &[&TraceRow]| {
        (0..rows.len()).find_map(|i| {
            let x = rows[i];
            let speed = (x.vx * x.vx + x.vy * x.vy).sqrt();
            let before = if i > 0 { rows[i - 1].hp } else { x.hp };
            [10.0f32, 25.0].into_iter().find(|v| (speed - v).abs() < 1e-3).map(|v| (x.step as f32, v, before - x.hp))
        })
    };
    let (hu, hr) = (first_hit(u), first_hit(r));
    let get = |h: Option<(f32, f32, f32)>, k: usize| h.map_or(f32::NAN, |h| [h.0, h.1, h.2][k]);
    checks.push(Check { name: "first hit step", unity: get(hu, 0), sim: get(hr, 0), tolerance: 1.0 });
    checks.push(Check { name: "first hit speed", unity: get(hu, 1), sim: get(hr, 1), tolerance: 0.0 });
    checks.push(Check { name: "hp change at first hit", unity: get(hu, 2), sim: get(hr, 2), tolerance: 0.0 });
}

/// S9：踏みつけでダメージが入るか。試合で最初に入ったダメージの発生ステップ ±1 と、受けた側が一致。
/// その後の衝突はカオス的に分かれるので比べない
fn stomp(unity: &[TraceRow], sim: &[TraceRow], checks: &mut Vec<Check>) {
    let first = |rows: &[TraceRow]| {
        let start = [rows[0].hp, rows[1].hp];
        rows.iter().find(|x| x.hp < start[x.slot as usize - 1]).map_or((f32::NAN, f32::NAN), |x| (x.step as f32, x.slot as f32))
    };
    let (u, r) = (first(unity), first(sim));
    checks.push(Check { name: "first damage step", unity: u.0, sim: r.0, tolerance: 1.0 });
    checks.push(Check { name: "first damaged slot", unity: u.1, sim: r.1, tolerance: 0.0 });
}

/// S10：場外になるステップ ±2、HP-1（最初にHPが減ったときだけを比べる）
fn hp_loss(u: &[&TraceRow], r: &[&TraceRow], step_tolerance: f32, checks: &mut Vec<Check>) {
    let first_loss = |rows: &[&TraceRow]| {
        (1..rows.len()).find(|&i| rows[i].hp < rows[i - 1].hp).map(|i| (rows[i].step as f32, rows[i - 1].hp - rows[i].hp))
    };
    let (lu, lr) = (first_loss(u), first_loss(r));
    let get = |l: Option<(f32, f32)>, k: usize| l.map_or(f32::NAN, |l| [l.0, l.1][k]);
    checks.push(Check { name: "first hp loss step", unity: get(lu, 0), sim: get(lr, 0), tolerance: step_tolerance });
    checks.push(Check { name: "first hp loss", unity: get(lu, 1), sim: get(lr, 1), tolerance: 0.0 });
}

/// S11：接地カウントの推移が一致（変化の回数が同じで、各変化のステップの差が ±1 以内）
fn ground_transitions(u: &[&TraceRow], r: &[&TraceRow], checks: &mut Vec<Check>) {
    let changes = |rows: &[&TraceRow]| -> Vec<(u32, u32)> {
        (1..rows.len()).filter(|&i| rows[i].ground != rows[i - 1].ground).map(|i| (rows[i].step, rows[i].ground)).collect()
    };
    let (cu, cr) = (changes(u), changes(r));
    checks.push(Check { name: "ground changes", unity: cu.len() as f32, sim: cr.len() as f32, tolerance: 0.0 });
    let offset = if cu.len() == cr.len() && cu.iter().zip(&cr).all(|(a, b)| a.1 == b.1) {
        cu.iter().zip(&cr).map(|(a, b)| a.0.abs_diff(b.0)).max().unwrap_or(0) as f32
    } else {
        f32::INFINITY
    };
    checks.push(Check { name: "ground change step offset", unity: 0.0, sim: offset, tolerance: 1.0 });
}

fn angle_diff(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

fn summary_line(r: &Result_) -> String {
    if let Some(e) = &r.error {
        return format!("{}: {e}", r.id);
    }
    let failed: Vec<String> = r
        .checks
        .iter()
        .filter(|c| !c.ok())
        .map(|c| format!("{} unity {} sim {}", c.name, c.unity, c.sim))
        .collect();
    let div = r.first_divergence.map_or("-".into(), |(s, slot)| format!("step {s} (P{slot})"));
    format!("{}: first divergence {div}; {}", r.id, failed.join("; "))
}

fn render(title: &str, results: &[Result_]) -> String {
    let mut md = String::new();
    let passed = results.iter().filter(|r| r.ok()).count();
    let _ = writeln!(md, "# 照合レポート {title}\n");
    let _ = writeln!(md, "{passed}/{} 合格。最初の{STRICT_STEPS}ステップは位置 {POS_TOL} 未満・角度 {ANGLE_TOL}° 未満で厳密に比較。\n", results.len());
    let _ = writeln!(md, "| シナリオ | 合否 | 最初にずれたステップ | 指標（unity / sim / 許容） |");
    let _ = writeln!(md, "| --- | --- | --- | --- |");
    for r in results {
        let div = r.first_divergence.map_or("-".into(), |(s, slot)| format!("{s} (P{slot})"));
        let checks = if let Some(e) = &r.error {
            e.clone()
        } else {
            r.checks
                .iter()
                .map(|c| format!("{}{}: {:.4} / {:.4} / ±{:.4}", if c.ok() { "" } else { "**✗** " }, c.name, c.unity, c.sim, c.tolerance))
                .collect::<Vec<_>>()
                .join("<br>")
        };
        let _ = writeln!(md, "| {} | {} | {div} | {checks} |", r.id, if r.ok() { "○" } else { "✗" });
    }
    md
}
