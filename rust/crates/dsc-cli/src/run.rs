//! `dsc run`：シナリオをシミュレーターで再生し、data/traces/sim-<ENGINE>/<id>.csv に書く。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use dsc_core::export::{CharacterExport, StageDir, StageExport};
use dsc_core::sim::{CppPhysics, Match, Physics, PlayerInput, RsPhysics, SimError, Snapshot};
use dsc_scenario::Scenario;
use dsc_scenario::trace::{self, Pose, TraceRow};

use crate::Options;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    /// box2d-rs（Box2D 2.4.1 の Rust 移植）
    Rs,
    /// C++ 版 Box2D 2.4.1
    Cpp,
}

impl Engine {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "rs" => Some(Self::Rs),
            "cpp" => Some(Self::Cpp),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Rs => "rs",
            Self::Cpp => "cpp",
        }
    }

    pub fn trace_dir(self, data_dir: &Path) -> PathBuf {
        data_dir.join("traces").join(format!("sim-{}", self.name()))
    }
}

pub fn run(data_dir: &Path, opts: &Options) -> Result<(), Box<dyn std::error::Error>> {
    let dir = StageDir::new(data_dir, &opts.stage);
    let export = dir.load_stage()?;
    let scenarios = load_scenarios(data_dir, opts)?;
    let characters = load_characters(&dir, &scenarios)?;
    let out = opts.engine.trace_dir(data_dir);
    std::fs::create_dir_all(&out)?;

    let errors: Vec<String> = parallel_map(&scenarios, |s| {
        let (p1, p2) = (&characters[&s.p1], &characters[&s.p2]);
        play_with(opts.engine, &export, p1, p2, s)
            .map_err(|e| e.to_string())
            .and_then(|rows| std::fs::write(out.join(format!("{}.csv", s.id)), trace::write(&rows)).map_err(|e| e.to_string()))
            .err()
            .map(|e| format!("{}: {e}", s.id))
    })
    .into_iter()
    .flatten()
    .collect();
    if !errors.is_empty() {
        return Err(errors.join("\n").into());
    }
    println!("ran {} scenarios with {} to {}", scenarios.len(), opts.engine.name(), out.display());
    Ok(())
}

/// 項目ごとに独立した処理を全コアで分け合う（物理ワールドは各スレッドの中で作る）。結果は items と同じ順
pub fn parallel_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(items.len().max(1));
    let mut indexed: Vec<(usize, R)> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                scope.spawn(|| {
                    let mut out = Vec::new();
                    let mut i = next.fetch_add(1, Ordering::Relaxed);
                    while let Some(item) = items.get(i) {
                        out.push((i, f(item)));
                        i = next.fetch_add(1, Ordering::Relaxed);
                    }
                    out
                })
            })
            .collect();
        workers.into_iter().flat_map(|w| w.join().expect("worker panicked")).collect()
    });
    indexed.sort_by_key(|(i, _)| *i);
    indexed.into_iter().map(|(_, r)| r).collect()
}

pub fn play_with(engine: Engine, stage: &StageExport, p1: &CharacterExport, p2: &CharacterExport, s: &Scenario) -> Result<Vec<TraceRow>, SimError> {
    match engine {
        Engine::Rs => play::<RsPhysics>(stage, p1, p2, s),
        Engine::Cpp => play::<CppPhysics>(stage, p1, p2, s),
    }
}

pub fn load_scenarios(data_dir: &Path, opts: &Options) -> Result<Vec<Scenario>, Box<dyn std::error::Error>> {
    let dir = data_dir.join("scenarios").join(&opts.stage);
    let mut scenarios = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let path = entry?.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let s = Scenario::from_json(&std::fs::read_to_string(&path)?)?;
        if opts.filter.as_ref().is_none_or(|f| f.is_match(&s.id)) {
            s.validate()?;
            scenarios.push(s);
        }
    }
    scenarios.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(scenarios)
}

pub fn load_characters(dir: &StageDir, scenarios: &[Scenario]) -> Result<HashMap<String, CharacterExport>, Box<dyn std::error::Error>> {
    let mut characters = HashMap::new();
    for s in scenarios {
        for name in [&s.p1, &s.p2] {
            if !characters.contains_key(name) {
                characters.insert(name.clone(), dir.load_character(name)?);
            }
        }
    }
    Ok(characters)
}

pub fn new_match<P: Physics>(stage: &StageExport, p1: &CharacterExport, p2: &CharacterExport, s: &Scenario) -> Result<Match<P>, SimError> {
    let mut m = Match::<P>::new(stage, p1, p2)?;
    for init in &s.init {
        m.apply_init(init.slot, init.pos, init.angle_deg, init.vel, init.ang_vel_deg);
    }
    Ok(m)
}

pub fn inputs(s: &Scenario, step: u32) -> [PlayerInput; 2] {
    [1, 2].map(|slot| {
        let i = s.input(slot, step);
        PlayerInput { move_x: i.move_x, rotate: i.rotate, jump: i.jump }
    })
}

/// 1本のシナリオを再生し、ステップごと・プレイヤーごとの状態を返す
pub fn play<P: Physics>(stage: &StageExport, p1: &CharacterExport, p2: &CharacterExport, s: &Scenario) -> Result<Vec<TraceRow>, SimError> {
    let mut m = new_match::<P>(stage, p1, p2, s)?;
    let mut rows = Vec::with_capacity(s.steps as usize * 2);
    for step in 0..s.steps {
        m.step(inputs(s, step));
        for slot in [1, 2] {
            rows.push(row(step, m.snapshot(slot)));
        }
    }
    Ok(rows)
}

fn row(step: u32, s: Snapshot) -> TraceRow {
    TraceRow {
        step,
        slot: s.slot,
        x: s.pos[0],
        y: s.pos[1],
        // v1 のゲームでは Transform の回転を書き換えないので、見た目の角度は剛体の角度と同じ
        angle: s.angle_deg,
        rot: s.angle_deg,
        vx: s.vel[0],
        vy: s.vel[1],
        w: s.ang_vel_deg,
        ground: s.ground,
        pose: match s.pose {
            dsc_core::export::Pose::Charge => Pose::Charge,
            dsc_core::export::Pose::Ult => Pose::Ult,
            _ => Pose::Normal,
        },
        hp: s.hp,
        ult: s.ult,
    }
}
