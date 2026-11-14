//! E3：着地合わせ（仕様書「CPU調整」）。空中のランダムな位置・速度・角度から、地面の傾きに板を合わせ、
//! 車輪から着地できる割合を調べる。目標は 95% 以上。
//!
//! - 出発状態：P1は左のお椀、P2は右のお椀の上空。高さ・速度・角度・角速度はランダム（1本で2サンプル）
//! - 成功：最初に地面に触れたステップで、車輪が着いていて、胴体（MainCollision）は触れていない。
//!   板（AttackCollision）は、お椀の曲面では板を合わせていても車輪と同時に端が触れるので、触れてもよい
//! - 比べるもの：何もしない／真上に合わせる（仮の値）／着地点の傾きに合わせる（数値を探索）
//! - 探索：1段目 60組 × 400サンプル、2段目 上位5組 × 1200サンプル、選んだ組を別の1200サンプルで検証

use std::fmt::Write as _;
use std::path::Path;

use dsc_ai::landing::{LandingParams, Wheels, landing_rotate, landing_target, rotate_toward};
use dsc_ai::terrain::Terrain;
use dsc_core::export::{CharacterExport, Pose, ShapeKind, StageDir, StageExport};
use dsc_core::sim::{CppPhysics, Match, Physics, PlayerInput, RsPhysics};
use dsc_scenario::rng::Pcg32;

use crate::Options;
use crate::e2::observe;
use crate::run::{Engine, parallel_map};

/// 最初の接触を待つ最大ステップ数（4秒）
const BUDGET: u32 = 200;
const TARGET_RATE: f32 = 0.95;
const STAGE1_SETS: usize = 60;
const STAGE1_STARTS: u32 = 200;
const STAGE2_SETS: usize = 5;
const STAGE2_STARTS: u32 = 600;
const HOLDOUT_PHASE: u64 = 3;

#[derive(Clone, Copy)]
enum Control {
    None,
    Upright(LandingParams),
    Landing(LandingParams),
}

#[derive(Clone)]
struct Candidate {
    label: String,
    control: Control,
}

#[derive(Clone, Copy)]
struct Start {
    pos: [[f32; 2]; 2],
    vel: [[f32; 2]; 2],
    angle: [f32; 2],
    ang_vel: [f32; 2],
}

struct World<'a> {
    stage: &'a StageExport,
    export: &'a CharacterExport,
    terrain: &'a Terrain,
    wheels: Wheels,
}

pub fn e3(data_dir: &Path, opts: &Options) -> Result<bool, Box<dyn std::error::Error>> {
    let dir = StageDir::new(data_dir, &opts.stage);
    let stage = dir.load_stage()?;
    let names = dir.characters()?;
    let terrain = terrain(&stage);

    let references = [
        Candidate { label: "none".into(), control: Control::None },
        Candidate { label: "upright".into(), control: Control::Upright(LandingParams::default()) },
        Candidate { label: "default".into(), control: Control::Landing(LandingParams::default()) },
    ];
    let mut rng = Pcg32::new(4, 0);
    let random: Vec<Candidate> = (0..STAGE1_SETS)
        .map(|i| Candidate { label: format!("r{i:03}"), control: Control::Landing(sample_params(&mut rng)) })
        .collect();
    let stage1: Vec<Candidate> = references.iter().cloned().chain(random).collect();

    let mut md = String::new();
    let _ = writeln!(md, "# E3 着地合わせ（{} / {}）\n", opts.stage, opts.engine.name());
    let _ = writeln!(
        md,
        "空中のランダムな出発状態から、最初に地面に触れたとき車輪から着いた割合。目標 {:.0}%。2段目・検証は各 {} サンプル。\n",
        TARGET_RATE * 100.0,
        STAGE2_STARTS * 2
    );
    let _ = writeln!(md, "| キャラ | 何もしない | 真上に合わせる | 傾きに合わせる（仮の値） | 選んだ組 | 2段目 | 検証 |");
    let _ = writeln!(md, "| --- | --- | --- | --- | --- | --- | --- |");
    let mut toml = String::from("# E3 で選んだ着地合わせの数値（dsc exp e3 が生成）\n");
    let mut samples_csv = String::from("character,success,steps_to_contact,first_error_deg,final_error_deg\n");
    let mut all_ok = true;

    for (c, name) in names.iter().enumerate() {
        let export = dir.load_character(name)?;
        let world = World { stage: &stage, export: &export, terrain: &terrain, wheels: wheels(&export) };

        let rates1 = evaluate(opts.engine, &world, &stage1, &starts(c as u64, 1, STAGE1_STARTS, &stage));
        let mut order: Vec<usize> = (references.len()..stage1.len()).collect();
        order.sort_by(|&a, &b| rates1[b].total_cmp(&rates1[a]));
        let finalists: Vec<Candidate> =
            references.iter().cloned().chain(order.iter().take(STAGE2_SETS).map(|&i| stage1[i].clone())).collect();
        let rates2 = evaluate(opts.engine, &world, &finalists, &starts(c as u64, 2, STAGE2_STARTS, &stage));
        let pick = (references.len()..finalists.len()).max_by(|&a, &b| rates2[a].total_cmp(&rates2[b])).expect("finalists");
        let holdout_samples = run_all(opts.engine, &world, std::slice::from_ref(&finalists[pick]), &starts(c as u64, HOLDOUT_PHASE, STAGE2_STARTS, &stage)).remove(0);
        let holdout = holdout_samples.iter().filter(|s| s.success).count() as f32 / holdout_samples.len() as f32;
        for s in &holdout_samples {
            let _ = writeln!(samples_csv, "{name},{},{},{:.1},{:.1}", s.success, s.steps, s.first_error, s.final_error);
        }

        let Control::Landing(p) = finalists[pick].control else { unreachable!("picked a landing controller") };
        let pct = |r: f32| format!("{}{:.1}%", if r >= TARGET_RATE { "" } else { "**✗** " }, r * 100.0);
        let _ = writeln!(
            md,
            "| {name} | {:.1}% | {:.1}% | {:.1}% | {}（gain {:.2}, damping {:.2}, 幅 {:.1}°） | {} | {} |",
            rates2[0] * 100.0,
            rates2[1] * 100.0,
            rates2[2] * 100.0,
            finalists[pick].label,
            p.gain,
            p.damping,
            p.deadband_deg,
            pct(rates2[pick]),
            pct(holdout)
        );
        let _ = writeln!(toml, "\n[{name}]\ngain = {}\ndamping = {}\ndeadband_deg = {}", p.gain, p.damping, p.deadband_deg);
        println!(
            "{name}: none {:.1}% upright {:.1}% landing {:.1}% (holdout {:.1}%)",
            rates2[0] * 100.0,
            rates2[1] * 100.0,
            rates2[pick] * 100.0,
            holdout * 100.0
        );
        all_ok &= holdout >= TARGET_RATE;
    }

    let out = data_dir.join("results");
    std::fs::create_dir_all(&out)?;
    let path = out.join(format!("e3-{}.md", opts.stage));
    std::fs::write(&path, md)?;
    std::fs::write(out.join(format!("e3-{}-best.toml", opts.stage)), toml)?;
    std::fs::write(out.join(format!("e3-{}-samples.csv", opts.stage)), samples_csv)?;
    println!("report: {}", path.display());
    Ok(all_ok)
}

/// ステージの当たり判定（多角形）から地形を作る
pub(crate) fn terrain(stage: &StageExport) -> Terrain {
    Terrain::from_polygons(stage.shapes.iter().filter(|s| s.kind == ShapeKind::Polygon).map(|s| s.vertices.as_slice()))
}

/// 通常の姿勢でまっすぐ立ったときの、体の中心から車輪の下端までの距離
pub(crate) fn wheels(export: &CharacterExport) -> Wheels {
    let slot = export.slot(1).expect("slot 1");
    let ride_height = slot
        .shapes
        .iter()
        .filter(|s| s.pose == Pose::Normal && s.tag == "TireCollision" && s.kind == ShapeKind::Circle)
        .map(|s| s.radius - s.vertices[0][1])
        .fold(0.0, f32::max);
    Wheels { ride_height }
}

fn sample_params(rng: &mut Pcg32) -> LandingParams {
    LandingParams { gain: (30f32.ln() * rng.unit()).exp(), damping: 2.0 * rng.unit(), deadband_deg: 10.0 * rng.unit() }
}

/// P1は左のお椀、P2は右のお椀の上空（スポーン地点のx の符号で左右を決める）
fn starts(character: u64, phase: u64, count: u32, stage: &StageExport) -> Vec<Start> {
    let mut rng = Pcg32::new(5, character << 8 | phase);
    let mut u = |lo: f32, hi: f32| lo + (hi - lo) * rng.unit();
    let side = stage.spawn.map(|s| s[0].signum());
    (0..count)
        .map(|_| {
            let mut s = Start { pos: [[0.0; 2]; 2], vel: [[0.0; 2]; 2], angle: [0.0; 2], ang_vel: [0.0; 2] };
            for (i, side) in side.iter().enumerate() {
                s.pos[i] = [side * u(3.0, 6.3), u(-1.5, 2.0)];
                s.vel[i] = [u(-4.0, 4.0), u(-2.0, 6.0)];
                s.angle[i] = u(-180.0, 180.0);
                s.ang_vel[i] = u(-300.0, 300.0);
            }
            s
        })
        .collect()
}

fn evaluate(engine: Engine, world: &World, candidates: &[Candidate], starts: &[Start]) -> Vec<f32> {
    let per = run_all(engine, world, candidates, starts);
    per.iter().map(|v| v.iter().filter(|s| s.success).count() as f32 / v.len().max(1) as f32).collect()
}

/// 候補ごとの全サンプルの結果
fn run_all(engine: Engine, world: &World, candidates: &[Candidate], starts: &[Start]) -> Vec<Vec<Sample>> {
    let tasks: Vec<(usize, Start)> = (0..candidates.len()).flat_map(|c| starts.iter().map(move |&s| (c, s))).collect();
    let results = parallel_map(&tasks, |&(c, start)| match engine {
        Engine::Rs => attempt::<RsPhysics>(world, candidates[c].control, start),
        Engine::Cpp => attempt::<CppPhysics>(world, candidates[c].control, start),
    });
    let mut per = vec![Vec::new(); candidates.len()];
    for (&(c, _), r) in tasks.iter().zip(results) {
        per[c].extend(r);
    }
    per
}

/// 1プレイヤーぶんの結果
#[derive(Debug, Clone, Copy, Default)]
struct Sample {
    success: bool,
    /// 最初に地面に触れるまでのステップ数（触れなければ BUDGET）
    steps: u32,
    /// 出発時点で、予測した着地点の傾きまで回す必要があった角度（度）
    first_error: f32,
    /// 触れた時点の、真下の地面の傾きとの角度の差（度）
    final_error: f32,
}

/// 1本ぶん：両プレイヤーが車輪から着地したか
fn attempt<P: Physics>(world: &World, control: Control, start: Start) -> [Sample; 2] {
    let Ok(mut m) = Match::<P>::new(world.stage, world.export, world.export) else { return [Sample::default(); 2] };
    for i in 0..2 {
        m.apply_init(i as u8 + 1, Some(start.pos[i]), Some(start.angle[i]), Some(start.vel[i]), Some(start.ang_vel[i]));
    }
    let gravity_y = world.stage.world.gravity[1];
    let error_to = |angle: f32, target: f32| (target - angle + 180.0).rem_euclid(360.0) - 180.0;
    let mut samples = [0, 1].map(|i| {
        let me = observe(&m.snapshot(i as u8 + 1));
        let target = landing_target(&me, world.terrain, gravity_y, world.wheels);
        Sample { steps: BUDGET, first_error: error_to(me.angle_deg, target), ..Default::default() }
    });
    let mut result: [Option<bool>; 2] = [None; 2];
    for step in 0..BUDGET {
        let mut inputs = [PlayerInput::default(); 2];
        for (i, input) in inputs.iter_mut().enumerate() {
            if result[i].is_some() {
                continue;
            }
            let me = observe(&m.snapshot(i as u8 + 1));
            input.rotate = match control {
                Control::None => 0.0,
                Control::Upright(p) => rotate_toward(&p, &me, 0.0),
                Control::Landing(p) => landing_rotate(&p, &me, world.terrain, gravity_y, world.wheels),
            };
        }
        m.step(inputs);
        for (i, r) in result.iter_mut().enumerate() {
            let s = m.snapshot(i as u8 + 1);
            if r.is_none() && (s.ground > 0 || s.body_ground > 0 || s.deck_ground > 0) {
                *r = Some(s.ground > 0 && s.body_ground == 0);
                let surface = world.terrain.floor_below(s.pos[0], s.pos[1]).map_or(0.0, |(_, a)| a);
                samples[i] = Sample { success: s.ground > 0 && s.body_ground == 0, steps: step + 1, final_error: error_to(s.angle_deg, surface), ..samples[i] };
            }
        }
        if result.iter().all(Option::is_some) {
            break;
        }
    }
    samples
}
