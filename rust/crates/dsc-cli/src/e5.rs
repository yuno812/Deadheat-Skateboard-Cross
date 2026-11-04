//! E5：対戦（仕様書「CPU調整」）。CPU（dsc_ai::brain）を基準の相手（何もしない／ランダム／突進しかしない）と
//! CPU 同士で戦わせ、キャラの組み合わせごとの勝率を出す。目標はランダム相手に勝率 95% 以上。
//!
//! - CPU の数値：E2・E3 で選んだもの（data/results/e2-<STAGE>-best.toml、e3-<STAGE>-best.toml）。なければ仮の値
//! - 組み合わせ：CPU のキャラ × 相手のキャラ、P1・P2 の両方。物理も CPU も決定的なので、
//!   何もしない・突進は1試合ずつ、ランダムは種を変えて RANDOM_GAMES 試合ずつ
//! - 試合：どちらかの HP が 0 になるか、60 秒で打ち切り（引き分け）

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use dsc_ai::brain::{Action, Brain, Knowledge, Tactics};
use dsc_ai::jump::JumpModel;
use dsc_ai::landing::LandingParams;
use dsc_ai::recovery::RecoveryParams;
use dsc_ai::terrain::Terrain;
use dsc_ai::tuned::tuned;
use dsc_ai::{BodyObs, Observation, Policy, Vec2};
use dsc_core::export::{CharacterExport, Pose, StageDir, StageExport};
use dsc_core::sim::{CppPhysics, Match, Physics, PlayerInput, RsPhysics};
use dsc_scenario::rng::Pcg32;

use crate::Options;
use crate::e2::observe;
use crate::e3::{terrain, wheels};
use crate::run::{Engine, parallel_map};

const MAX_STEPS: u32 = 3000;
pub(crate) const RANDOM_GAMES: u64 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Opponent {
    Idle,
    Random,
    Rush,
    Cpu,
}

impl Opponent {
    fn name(self) -> &'static str {
        match self {
            Opponent::Idle => "何もしない",
            Opponent::Random => "ランダム",
            Opponent::Rush => "突進",
            Opponent::Cpu => "CPU",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Game {
    pub(crate) cpu: usize,
    pub(crate) opp: usize,
    pub(crate) opponent: Opponent,
    /// CPU が P1（true）か P2 か
    pub(crate) cpu_is_p1: bool,
    pub(crate) seed: u64,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Outcome {
    /// CPU から見て 1 = 勝ち、0 = 引き分け、-1 = 負け
    pub(crate) result: i8,
    pub(crate) steps: u32,
    pub(crate) dealt: f32,
    pub(crate) taken: f32,
    /// CPU の行動の内訳（ステップ数）
    pub(crate) actions: [u32; 5],
    /// CPU がダメージを受けたとき、どの行動をしていたか（回数）
    pub(crate) hurt: [u32; 5],
}

enum Agent {
    Cpu(Box<Brain>),
    Idle,
    Random { rng: Pcg32, input: PlayerInput, left: u32 },
    Rush,
}

impl Agent {
    fn act(&mut self, obs: &Observation) -> PlayerInput {
        match self {
            Agent::Cpu(brain) => {
                let i = brain.act(obs);
                PlayerInput { move_x: i.move_x, rotate: i.rotate, jump: i.jump }
            }
            Agent::Idle => PlayerInput::default(),
            Agent::Random { rng, input, left } => {
                // R1 と同じ：10〜50 ステップごとに、前後・回転を -1/0/1、ジャンプの押しっぱなしを 30% で選び直す
                if *left == 0 {
                    *left = 10 + rng.below(41);
                    *input = PlayerInput {
                        move_x: rng.below(3) as f32 - 1.0,
                        rotate: rng.below(3) as f32 - 1.0,
                        jump: rng.below(10) < 3,
                    };
                }
                *left -= 1;
                *input
            }
            Agent::Rush => {
                let dir = (obs.opp.pos.x - obs.me.pos.x).signum();
                PlayerInput { move_x: dir * obs.me.facing, ..Default::default() }
            }
        }
    }
}

pub(crate) struct Setup<'a> {
    pub(crate) tactics: Tactics,
    pub(crate) stage: &'a StageExport,
    pub(crate) exports: &'a [CharacterExport],
    pub(crate) knowledge: &'a [Knowledge],
    pub(crate) terrain: &'a Terrain,
}

pub fn e5(data_dir: &Path, opts: &Options) -> Result<bool, Box<dyn std::error::Error>> {
    let dir = StageDir::new(data_dir, &opts.stage);
    let stage = dir.load_stage()?;
    let names = dir.characters()?;
    let exports: Vec<CharacterExport> = names.iter().map(|n| dir.load_character(n)).collect::<Result<_, _>>()?;
    let knowledge = knowledge(data_dir, &opts.stage, &stage, &names, &exports);
    let terrain = terrain(&stage);
    let setup = Setup { tactics: load_tactics(data_dir, &opts.stage), stage: &stage, exports: &exports, knowledge: &knowledge, terrain: &terrain };

    let n = names.len();
    let mut games = Vec::new();
    for opponent in [Opponent::Idle, Opponent::Random, Opponent::Rush, Opponent::Cpu] {
        let seeds = if opponent == Opponent::Random { RANDOM_GAMES } else { 1 };
        for cpu in 0..n {
            for opp in 0..n {
                for cpu_is_p1 in [true, false] {
                    for seed in 0..seeds {
                        games.push(Game { cpu, opp, opponent, cpu_is_p1, seed });
                    }
                }
            }
        }
    }
    let outcomes = parallel_map(&games, |g| match opts.engine {
        Engine::Rs => play::<RsPhysics>(&setup, *g),
        Engine::Cpp => play::<CppPhysics>(&setup, *g),
    });

    let md = report(&opts.stage, opts.engine.name(), &names, &games, &outcomes);
    let out = data_dir.join("results");
    std::fs::create_dir_all(&out)?;
    let path = out.join(format!("e5-{}.md", opts.stage));
    std::fs::write(&path, &md)?;
    println!("{}", md.lines().take_while(|l| !l.starts_with("## ")).collect::<Vec<_>>().join("\n"));
    println!("report: {}", path.display());
    Ok(true)
}

/// e5-search で選んだ行動の数値（data/results/e5-<STAGE>-tactics.toml）。
/// なければ、ゲームの CPU と同じ調整済みの値（rust/profiles/tuned-R2500.toml）
fn load_tactics(data_dir: &Path, stage: &str) -> Tactics {
    let path = data_dir.join("results").join(format!("e5-{stage}-tactics.toml"));
    match std::fs::read_to_string(&path).map(|t| toml::from_str(&t)) {
        Ok(Ok(t)) => t,
        _ => {
            eprintln!("note: {} が無いので、調整済みの値（tuned-R2500.toml）の行動の数値を使う", path.display());
            tuned().tactics
        }
    }
}

fn load_toml<T: serde::de::DeserializeOwned>(path: &Path) -> BTreeMap<String, T> {
    match std::fs::read_to_string(path).map(|t| toml::from_str(&t)) {
        Ok(Ok(v)) => v,
        _ => {
            eprintln!("warning: {} を読めないので、調整済みの値（tuned-R2500.toml）を使う", path.display());
            BTreeMap::new()
        }
    }
}

fn agent(setup: &Setup, opponent: Opponent, character: usize, seed: u64) -> Agent {
    let cpu = || Agent::Cpu(Box::new(Brain::new(setup.knowledge[character], setup.tactics, setup.terrain.clone())));
    match opponent {
        Opponent::Idle => Agent::Idle,
        Opponent::Random => Agent::Random { rng: Pcg32::new(7, seed), input: PlayerInput::default(), left: 0 },
        Opponent::Rush => Agent::Rush,
        Opponent::Cpu => cpu(),
    }
}

pub(crate) fn play<P: Physics>(setup: &Setup, g: Game) -> Outcome {
    play_traced::<P>(setup, g, &mut |_, _, _| {})
}

/// 1試合。on_step はステップごとに（ステップ番号、両プレイヤーの状態、CPU の行動）で呼ばれる
fn play_traced<P: Physics>(setup: &Setup, g: Game, on_step: &mut dyn FnMut(u32, [BodyObs; 2], Option<Action>)) -> Outcome {
    let (p1, p2) = if g.cpu_is_p1 { (g.cpu, g.opp) } else { (g.opp, g.cpu) };
    let Ok(mut m) = Match::<P>::new(setup.stage, &setup.exports[p1], &setup.exports[p2]) else { return Outcome::default() };
    let cpu_agent = Agent::Cpu(Box::new(Brain::new(setup.knowledge[g.cpu], setup.tactics, setup.terrain.clone())));
    let opp_agent = agent(setup, g.opponent, g.opp, g.seed);
    let mut agents = if g.cpu_is_p1 { [cpu_agent, opp_agent] } else { [opp_agent, cpu_agent] };
    let cpu_slot = if g.cpu_is_p1 { 0 } else { 1 };
    let bounds = Vec2 { x: setup.stage.out_of_area[0], y: setup.stage.out_of_area[1] };

    let start_hp = [m.snapshot(1).hp, m.snapshot(2).hp];
    let mut out = Outcome { steps: MAX_STEPS, ..Default::default() };
    for step in 0..MAX_STEPS {
        let bodies: [BodyObs; 2] = [observe(&m.snapshot(1)), observe(&m.snapshot(2))];
        let mut inputs = [PlayerInput::default(); 2];
        for i in 0..2 {
            let obs = Observation { step, me: bodies[i], opp: bodies[1 - i], gravity_up: Vec2 { x: 0.0, y: 1.0 }, bounds };
            inputs[i] = agents[i].act(&obs);
        }
        let action = if let Agent::Cpu(brain) = &agents[cpu_slot] { Some(brain.last_action()) } else { None };
        if let Some(a) = action {
            out.actions[action_index(a)] += 1;
        }
        on_step(step, bodies, action);
        let cpu_hp = m.snapshot(cpu_slot as u8 + 1).hp;
        m.step(inputs);
        let hp = [m.snapshot(1).hp, m.snapshot(2).hp];
        if let Some(a) = action
            && hp[cpu_slot] < cpu_hp
        {
            out.hurt[action_index(a)] += 1;
        }
        if hp[0] <= 0.0 || hp[1] <= 0.0 {
            out.steps = step + 1;
            break;
        }
    }
    let hp = [m.snapshot(1).hp, m.snapshot(2).hp];
    let (me, them) = (cpu_slot, 1 - cpu_slot);
    out.dealt = start_hp[them] - hp[them];
    out.taken = start_hp[me] - hp[me];
    out.result = match (hp[me] <= 0.0, hp[them] <= 0.0) {
        (false, true) => 1,
        (true, false) => -1,
        _ => 0,
    };
    out
}

fn action_index(a: Action) -> usize {
    match a {
        Action::Land => 0,
        Action::Recover => 1,
        Action::StayInBounds => 2,
        Action::Dodge => 3,
        Action::Aim => 4,
    }
}

fn report(stage: &str, engine: &str, names: &[String], games: &[Game], outcomes: &[Outcome]) -> String {
    let mut md = String::new();
    let _ = writeln!(md, "# E5 対戦（{stage} / {engine}）\n");
    let _ = writeln!(md, "| 相手 | 試合 | CPU の勝ち | 引き分け | 負け | 与えたダメージ / 受けたダメージ（1試合平均） | 試合時間の中央値 |");
    let _ = writeln!(md, "| --- | --- | --- | --- | --- | --- | --- |");
    for opponent in [Opponent::Idle, Opponent::Random, Opponent::Rush, Opponent::Cpu] {
        let v: Vec<&Outcome> = games.iter().zip(outcomes).filter(|(g, _)| g.opponent == opponent).map(|(_, o)| o).collect();
        let count = |r: i8| v.iter().filter(|o| o.result == r).count() as f32 / v.len().max(1) as f32 * 100.0;
        let mean = |f: &dyn Fn(&Outcome) -> f32| v.iter().map(|o| f(o)).sum::<f32>() / v.len().max(1) as f32;
        let mut steps: Vec<u32> = v.iter().map(|o| o.steps).collect();
        steps.sort_unstable();
        let _ = writeln!(
            md,
            "| {} | {} | {:.1}% | {:.1}% | {:.1}% | {:.2} / {:.2} | {:.1}秒 |",
            opponent.name(),
            v.len(),
            count(1),
            count(0),
            count(-1),
            mean(&|o| o.dealt),
            mean(&|o| o.taken),
            steps.get(steps.len() / 2).copied().unwrap_or(0) as f32 * 0.02
        );
    }

    // CPU の行動の内訳
    let mut actions = [0u64; 5];
    for o in outcomes {
        for (a, &n) in actions.iter_mut().zip(&o.actions) {
            *a += n as u64;
        }
    }
    let total = actions.iter().sum::<u64>().max(1) as f32;
    let labels = ["空中", "復帰", "場外回避", "防御", "狙って跳ぶ"];
    let _ = writeln!(
        md,
        "\nCPU の行動の内訳：{}\n",
        labels.iter().zip(actions).map(|(l, a)| format!("{l} {:.1}%", a as f32 / total * 100.0)).collect::<Vec<_>>().join("、")
    );
    let mut hurt = [0u64; 5];
    for o in outcomes {
        for (h, &n) in hurt.iter_mut().zip(&o.hurt) {
            *h += n as u64;
        }
    }
    let _ = writeln!(
        md,
        "CPU がダメージを受けたときの行動：{}\n",
        labels.iter().zip(hurt).map(|(l, h)| format!("{l} {h}回")).collect::<Vec<_>>().join("、")
    );

    for opponent in [Opponent::Random, Opponent::Rush, Opponent::Cpu] {
        let _ = writeln!(md, "## 勝率：CPU（行） 対 {}（列）\n", opponent.name());
        let _ = writeln!(md, "| CPU ＼ 相手 | {} |", names.join(" | "));
        let _ = writeln!(md, "| --- |{}", " --- |".repeat(names.len()));
        for (c, name) in names.iter().enumerate() {
            let cells: Vec<String> = (0..names.len())
                .map(|o| {
                    let v: Vec<&Outcome> = games
                        .iter()
                        .zip(outcomes)
                        .filter(|(g, _)| g.opponent == opponent && g.cpu == c && g.opp == o)
                        .map(|(_, r)| r)
                        .collect();
                    let w = v.iter().filter(|r| r.result == 1).count();
                    let d = v.iter().filter(|r| r.result == 0).count();
                    format!("{w}勝{d}分/{}", v.len())
                })
                .collect();
            let _ = writeln!(md, "| {name} | {} |", cells.join(" | "));
        }
        md.push('\n');
    }
    md
}

/// `dsc exp e5-trace CPU_CHAR OPP_CHAR idle|random|rush|cpu`：1試合の経過を 10 ステップごとに出す（調査用）
pub fn trace(data_dir: &Path, opts: &Options, cpu: &str, opp: &str, opponent: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let dir = StageDir::new(data_dir, &opts.stage);
    let stage = dir.load_stage()?;
    let names = dir.characters()?;
    let index = |n: &str| names.iter().position(|x| x == n).ok_or_else(|| format!("unknown character {n}"));
    let (cpu, opp) = (index(cpu)?, index(opp)?);
    let opponent = match opponent {
        "idle" => Opponent::Idle,
        "random" => Opponent::Random,
        "rush" => Opponent::Rush,
        "cpu" => Opponent::Cpu,
        other => return Err(format!("unknown opponent {other}").into()),
    };
    let exports: Vec<CharacterExport> = names.iter().map(|n| dir.load_character(n)).collect::<Result<_, _>>()?;
    let knowledge = knowledge(data_dir, &opts.stage, &stage, &names, &exports);
    let terrain = terrain(&stage);
    let setup = Setup { tactics: load_tactics(data_dir, &opts.stage), stage: &stage, exports: &exports, knowledge: &knowledge, terrain: &terrain };
    println!("step  cpu: x y angle ground vx action | opp: x y angle ground");
    let o = play_traced::<CppPhysics>(&setup, Game { cpu, opp, opponent, cpu_is_p1: true, seed: 0 }, &mut |step, b, a| {
        if step % 10 == 0 {
            println!(
                "{step:5} {:6.2} {:6.2} {:7.1} {} {:6.2} {:?} | {:6.2} {:6.2} {:7.1} {}",
                b[0].pos.x, b[0].pos.y, b[0].angle_deg, b[0].ground_count, b[0].vel.x, a, b[1].pos.x, b[1].pos.y, b[1].angle_deg, b[1].ground_count
            );
        }
    });
    println!("result {} after {} steps, dealt {} taken {}", o.result, o.steps, o.dealt, o.taken);
    Ok(true)
}

pub(crate) fn knowledge(data_dir: &Path, stage_name: &str, stage: &StageExport, names: &[String], exports: &[CharacterExport]) -> Vec<Knowledge> {
    let results = data_dir.join("results");
    let recovery: BTreeMap<String, RecoveryParams> = load_toml(&results.join(format!("e2-{stage_name}-best.toml")));
    let landing: BTreeMap<String, LandingParams> = load_toml(&results.join(format!("e3-{stage_name}-best.toml")));
    names
        .iter()
        .zip(exports)
        .map(|(n, e)| Knowledge {
            // 実験の結果が無ければ、ゲームの CPU と同じ調整済みの値（tuned-R2500.toml）
            recovery: recovery.get(n).copied().or_else(|| tuned().characters.get(n).map(|c| c.recovery)).unwrap_or_default(),
            landing: landing.get(n).copied().or_else(|| tuned().characters.get(n).map(|c| c.landing)).unwrap_or_default(),
            wheels: wheels(e),
            jump: JumpModel {
                jump_force: e.stats.jump_force,
                max_speed: e.stats.max_speed,
                deceleration: e.stats.deceleration,
                center_of_mass: e.slot(1).map_or([0.0; 2], |s| s.poses[&Pose::Normal].center_of_mass),
                gravity_y: stage.world.gravity[1],
                dt: stage.world.dt,
            },
        })
        .collect()
}
