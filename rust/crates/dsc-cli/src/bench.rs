//! `dsc bench`：シナリオの再生速度を測る（試合の組み立てを除いた、ステップだけの速さ）。
//! 仕様書の目標は 1コアあたり毎秒5万ステップ以上。

use std::path::Path;
use std::time::{Duration, Instant};

use dsc_core::export::{CharacterExport, StageDir, StageExport};
use dsc_core::sim::{CppPhysics, Physics, RsPhysics};
use dsc_scenario::Scenario;

use crate::Options;
use crate::run::{Engine, inputs, load_characters, load_scenarios, new_match};

#[derive(Default)]
struct Totals {
    build: Duration,
    stepping: Duration,
    steps: u64,
    contacts: u64,
}

pub fn bench(data_dir: &Path, opts: &Options) -> Result<(), Box<dyn std::error::Error>> {
    let dir = StageDir::new(data_dir, &opts.stage);
    let export = dir.load_stage()?;
    let scenarios = load_scenarios(data_dir, opts)?;
    let characters = load_characters(&dir, &scenarios)?;

    let mut t = Totals::default();
    for s in &scenarios {
        let (p1, p2) = (&characters[&s.p1], &characters[&s.p2]);
        match opts.engine {
            Engine::Rs => measure::<RsPhysics>(&export, p1, p2, s, &mut t)?,
            Engine::Cpp => measure::<CppPhysics>(&export, p1, p2, s, &mut t)?,
        }
    }

    println!("{}: {} scenarios, {} steps (1 step = both players)", opts.engine.name(), scenarios.len(), t.steps);
    println!("  contacts per step: {:.0}", t.contacts as f64 / t.steps as f64);
    println!("  build:    {:>8.1} ms total, {:.2} ms per match", ms(t.build), ms(t.build) / scenarios.len() as f64);
    println!("  stepping: {:>8.1} ms total, {:.0} steps/s on one core", ms(t.stepping), t.steps as f64 / t.stepping.as_secs_f64());
    Ok(())
}

fn measure<P: Physics>(stage: &StageExport, p1: &CharacterExport, p2: &CharacterExport, s: &Scenario, t: &mut Totals) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    let mut m = new_match::<P>(stage, p1, p2, s)?;
    t.build += start.elapsed();

    let start = Instant::now();
    for step in 0..s.steps {
        m.step(inputs(s, step));
        t.contacts += m.contact_count() as u64;
    }
    t.stepping += start.elapsed();
    t.steps += s.steps as u64;
    Ok(())
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1e3
}
