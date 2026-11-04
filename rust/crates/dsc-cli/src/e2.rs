//! E2：復帰処理（dsc_ai::recovery）の数値をランダム探索する（仕様書「CPU調整」）。
//! キャラごとに、成功率 99% 以上で復帰時間の中央値が最小になる組を選ぶ。
//!
//! - 出発状態：±90〜180° のランダムな角度、±100°/秒の角速度で置き、落ち着かせてから復帰を始める。
//!   P1とP2に別々の出発状態を与える（1本で2サンプル）。どの数値の組も同じ出発状態で比べる
//! - 成功：両輪が着き、|角度| < 30°、|角速度| < 60°/秒 が 0.5 秒続いた。時間はその始まりまで。8秒で打ち切り
//! - 1段目：ランダムな120組を100本で絞り込み、2段目：上位8組を別の400本で比べて決める。
//!   比較のため、仮の値（Default）と E1 の「真上付近で離すだけ」も入れる

use std::fmt::Write as _;
use std::path::Path;

use dsc_ai::recovery::{Recovery, RecoveryParams};
use dsc_ai::{BodyObs, Vec2};
use dsc_core::export::{CharacterExport, StageDir, StageExport};
use dsc_core::sim::{CppPhysics, Match, Physics, PlayerInput, RsPhysics, Snapshot};
use dsc_scenario::rng::Pcg32;

use crate::Options;
use crate::run::{Engine, parallel_map};

const SETTLE: u32 = 50;
/// 復帰を始めてからの打ち切り（8秒）
const BUDGET: u32 = 400;
/// この状態が続いたら「起き上がった」
const STABLE_STEPS: u32 = 25;
const STABLE_ANGLE: f32 = 30.0;
const STABLE_ANG_VEL: f32 = 60.0;
const TARGET_RATE: f32 = 0.99;
const DT: f32 = 0.02;

const STAGE1_SETS: usize = 120;
const STAGE1_STARTS: u32 = 100;
const STAGE2_SETS: usize = 8;
const STAGE2_STARTS: u32 = 400;

#[derive(Clone)]
struct Candidate {
    label: String,
    params: RecoveryParams,
}

#[derive(Clone, Copy)]
struct Start {
    angle: [f32; 2],
    ang_vel: [f32; 2],
}

struct Score {
    rate: f32,
    median: f32,
    p90: f32,
    samples: usize,
}

impl Score {
    fn new(times: &[Option<u32>]) -> Self {
        let mut ok: Vec<u32> = times.iter().flatten().copied().collect();
        ok.sort_unstable();
        let pick = |q: f32| ok.get(((ok.len() as f32 * q) as usize).min(ok.len().saturating_sub(1))).map_or(f32::NAN, |&s| s as f32 * DT);
        Score { rate: ok.len() as f32 / times.len().max(1) as f32, median: pick(0.5), p90: pick(0.9), samples: times.len() }
    }

    /// 成功率 99% 以上を先に、その中で中央値が小さい順。届かないものは成功率が高い順
    fn rank_key(&self) -> (u8, f32) {
        if self.rate >= TARGET_RATE { (0, self.median) } else { (1, -self.rate) }
    }
}

pub fn e2(data_dir: &Path, opts: &Options) -> Result<bool, Box<dyn std::error::Error>> {
    let dir = StageDir::new(data_dir, &opts.stage);
    let stage = dir.load_stage()?;
    let names = dir.characters()?;
    let characters: Vec<CharacterExport> = names.iter().map(|n| dir.load_character(n)).collect::<Result<_, _>>()?;

    let references = vec![
        Candidate { label: "default".into(), params: RecoveryParams::default() },
        // E1 の B と同じ「真上付近まで回し続けて離すだけ」
        Candidate {
            label: "release-only".into(),
            params: RecoveryParams { gain: 1000.0, damping: 0.0, deadband_deg: 30.0, stall_steps: 10_000, timeout_steps: 10_000 },
        },
    ];
    let mut rng = Pcg32::new(3, 0);
    let random: Vec<Candidate> = (0..STAGE1_SETS).map(|i| Candidate { label: format!("r{i:03}"), params: sample_params(&mut rng) }).collect();
    let stage1: Vec<Candidate> = references.iter().cloned().chain(random).collect();

    let mut csv = String::from("phase,character,label,gain,damping,deadband_deg,stall_steps,timeout_steps,samples,rate,median_s,p90_s\n");
    let mut best = Vec::new();
    let mut md = String::new();
    let _ = writeln!(md, "# E2 復帰処理の数値探索（{} / {}）\n", opts.stage, opts.engine.name());
    let _ = writeln!(
        md,
        "成功率 {:.0}% 以上で、復帰時間の中央値が最小の組を選ぶ。1段目 {STAGE1_SETS} 組 × {} サンプル、2段目 上位 {STAGE2_SETS} 組 × {} サンプル（別の出発状態）。\n",
        TARGET_RATE * 100.0,
        STAGE1_STARTS * 2,
        STAGE2_STARTS * 2
    );
    let _ = writeln!(md, "| キャラ | 選んだ組 | 成功率 | 中央値 | 90%点 | 仮の値（成功率・中央値） | 離すだけ（成功率・中央値） |");
    let _ = writeln!(md, "| --- | --- | --- | --- | --- | --- | --- |");

    for (c, name) in names.iter().enumerate() {
        let export = &characters[c];
        let scores1 = evaluate(opts.engine, &stage, export, &stage1, &starts(c as u64, 1, STAGE1_STARTS));
        for (cand, s) in stage1.iter().zip(&scores1) {
            write_csv(&mut csv, "stage1", name, cand, s);
        }

        let mut order: Vec<usize> = (0..stage1.len()).collect();
        order.sort_by(|&a, &b| scores1[a].rank_key().partial_cmp(&scores1[b].rank_key()).unwrap_or(std::cmp::Ordering::Equal));
        let mut finalists: Vec<Candidate> = order.iter().take(STAGE2_SETS).map(|&i| stage1[i].clone()).collect();
        for r in &references {
            if !finalists.iter().any(|f| f.label == r.label) {
                finalists.push(r.clone());
            }
        }
        let scores2 = evaluate(opts.engine, &stage, export, &finalists, &starts(c as u64, 2, STAGE2_STARTS));
        for (cand, s) in finalists.iter().zip(&scores2) {
            write_csv(&mut csv, "stage2", name, cand, s);
        }

        let pick = (0..finalists.len())
            .filter(|&i| !references.iter().any(|r| r.label == finalists[i].label))
            .min_by(|&a, &b| scores2[a].rank_key().partial_cmp(&scores2[b].rank_key()).unwrap_or(std::cmp::Ordering::Equal))
            .expect("finalists");
        let reference = |label: &str| {
            let i = finalists.iter().position(|f| f.label == label).expect("reference");
            format!("{:.1}%・{:.2}秒", scores2[i].rate * 100.0, scores2[i].median)
        };
        let (cand, s) = (&finalists[pick], &scores2[pick]);
        let p = cand.params;
        let _ = writeln!(
            md,
            "| {name} | {}（gain {:.2}, damping {:.2}, 幅 {:.1}°, 切り返し {}, 時間切れ {}） | {}{:.1}% | {:.2}秒 | {:.2}秒 | {} | {} |",
            cand.label,
            p.gain,
            p.damping,
            p.deadband_deg,
            p.stall_steps,
            p.timeout_steps,
            if s.rate >= TARGET_RATE { "" } else { "**✗** " },
            s.rate * 100.0,
            s.median,
            s.p90,
            reference("default"),
            reference("release-only")
        );
        println!("{name}: {} rate {:.1}% median {:.2}s", cand.label, s.rate * 100.0, s.median);
        best.push((name.clone(), p));
    }

    let out = data_dir.join("results");
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join(format!("e2-{}.csv", opts.stage)), csv)?;
    let path = out.join(format!("e2-{}.md", opts.stage));
    std::fs::write(&path, md)?;
    let mut toml = String::from("# E2 で選んだ復帰処理の数値（dsc exp e2 が生成）\n");
    for (name, p) in &best {
        let _ = writeln!(
            toml,
            "\n[{name}]\ngain = {}\ndamping = {}\ndeadband_deg = {}\nstall_steps = {}\ntimeout_steps = {}",
            p.gain, p.damping, p.deadband_deg, p.stall_steps, p.timeout_steps
        );
    }
    std::fs::write(out.join(format!("e2-{}-best.toml", opts.stage)), toml)?;
    println!("report: {}", path.display());
    Ok(true)
}

fn sample_params(rng: &mut Pcg32) -> RecoveryParams {
    RecoveryParams {
        // 1〜30 を対数一様に
        gain: (30f32.ln() * rng.unit()).exp(),
        damping: 2.0 * rng.unit(),
        deadband_deg: 30.0 * rng.unit(),
        stall_steps: 5 + rng.below(56),
        timeout_steps: 50 + rng.below(251),
    }
}

/// キャラと段ごとに決まった出発状態（どの数値の組も同じものを使う）
fn starts(character: u64, phase: u64, count: u32) -> Vec<Start> {
    let mut rng = Pcg32::new(2, character << 8 | phase);
    let mut angle = || {
        let a = 90.0 + 90.0 * rng.unit();
        if rng.below(2) == 0 { a } else { -a }
    };
    let pairs: Vec<[f32; 2]> = (0..count).map(|_| [angle(), angle()]).collect();
    let mut rng = Pcg32::new(2, character << 8 | phase | 0x80);
    pairs
        .into_iter()
        .map(|angle| Start { angle, ang_vel: [0, 1].map(|_| (rng.unit() * 2.0 - 1.0) * 100.0) })
        .collect()
}

fn evaluate(engine: Engine, stage: &StageExport, export: &CharacterExport, candidates: &[Candidate], starts: &[Start]) -> Vec<Score> {
    let tasks: Vec<(usize, Start)> = (0..candidates.len()).flat_map(|c| starts.iter().map(move |&s| (c, s))).collect();
    let times = parallel_map(&tasks, |&(c, start)| match engine {
        Engine::Rs => attempt::<RsPhysics>(stage, export, candidates[c].params, start),
        Engine::Cpp => attempt::<CppPhysics>(stage, export, candidates[c].params, start),
    });
    let mut per = vec![Vec::new(); candidates.len()];
    for (&(c, _), t) in tasks.iter().zip(times) {
        per[c].extend(t);
    }
    per.iter().map(|t| Score::new(t)).collect()
}

/// 1本ぶん：両プレイヤーが復帰処理で起き上がるまでのステップ数（打ち切りなら None）
fn attempt<P: Physics>(stage: &StageExport, export: &CharacterExport, params: RecoveryParams, start: Start) -> [Option<u32>; 2] {
    let Ok(mut m) = Match::<P>::new(stage, export, export) else { return [None; 2] };
    for slot in [1, 2] {
        let i = slot as usize - 1;
        m.apply_init(slot, None, Some(start.angle[i]), None, Some(start.ang_vel[i]));
    }
    for _ in 0..SETTLE {
        m.step([PlayerInput::default(); 2]);
    }

    let mut controllers = [Recovery::new(params), Recovery::new(params)];
    let mut stable = [0u32; 2];
    let mut done: [Option<u32>; 2] = [None; 2];
    for step in 0..BUDGET {
        let mut inputs = [PlayerInput::default(); 2];
        for i in 0..2 {
            if done[i].is_none() {
                inputs[i].rotate = controllers[i].update(&observe(&m.snapshot(i as u8 + 1)));
            }
        }
        m.step(inputs);
        for i in 0..2 {
            let s = m.snapshot(i as u8 + 1);
            let upright = s.ground >= 2 && s.angle_deg.abs() < STABLE_ANGLE && s.ang_vel_deg.abs() < STABLE_ANG_VEL;
            stable[i] = if upright { stable[i] + 1 } else { 0 };
            if done[i].is_none() && stable[i] >= STABLE_STEPS {
                done[i] = Some(step + 1 - STABLE_STEPS);
            }
        }
        if done.iter().all(Option::is_some) {
            break;
        }
    }
    done
}

fn observe(s: &Snapshot) -> BodyObs {
    BodyObs {
        pos: Vec2 { x: s.pos[0], y: s.pos[1] },
        vel: Vec2 { x: s.vel[0], y: s.vel[1] },
        angle_deg: s.angle_deg,
        ang_vel_deg: s.ang_vel_deg,
        ground_count: s.ground.min(255) as u8,
        hp: s.hp,
        ult: s.ult,
        facing: if s.slot == 1 { 1.0 } else { -1.0 },
        ..Default::default()
    }
}

fn write_csv(csv: &mut String, phase: &str, name: &str, c: &Candidate, s: &Score) {
    let p = c.params;
    let _ = writeln!(
        csv,
        "{phase},{name},{},{},{},{},{},{},{},{:.4},{:.3},{:.3}",
        c.label, p.gain, p.damping, p.deadband_deg, p.stall_steps, p.timeout_steps, s.samples, s.rate, s.median, s.p90
    );
}
