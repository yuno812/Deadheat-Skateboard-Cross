//! `dsc exp e5-search`：対戦用CPUの行動の数値（Tactics）をランダム探索し、ランダム相手の勝率が最も高い組を選ぶ。
//! 1段目 SEARCH_SETS 組 × 種2つ（196試合）、2段目 上位 FINALISTS 組 × 種5つ（490試合）、
//! 選んだ組を探索に使っていない種で検証する。選んだ値は data/results/e5-<STAGE>-tactics.toml（dsc exp e5 が読む）

use std::fmt::Write as _;
use std::path::Path;

use dsc_ai::brain::Tactics;
use dsc_core::export::{CharacterExport, StageDir};
use dsc_core::sim::{CppPhysics, RsPhysics};
use dsc_scenario::rng::Pcg32;

use crate::Options;
use crate::e3::terrain;
use crate::e5::{Game, Opponent, Outcome, RANDOM_GAMES, Setup, knowledge, play};
use crate::run::{Engine, parallel_map};

const SEARCH_SETS: usize = 40;
const FINALISTS: usize = 5;
/// 検証に使う乱数の種の始まり（探索では 0..RANDOM_GAMES を使う）
const HOLDOUT_SEED: u64 = 1000;

pub fn search(data_dir: &Path, opts: &Options) -> Result<bool, Box<dyn std::error::Error>> {
    let dir = StageDir::new(data_dir, &opts.stage);
    let stage = dir.load_stage()?;
    let names = dir.characters()?;
    let exports: Vec<CharacterExport> = names.iter().map(|n| dir.load_character(n)).collect::<Result<_, _>>()?;
    let knowledge = knowledge(data_dir, &opts.stage, &stage, &names, &exports);
    let terrain = terrain(&stage);

    let mut rng = Pcg32::new(8, 0);
    let mut labels = vec!["default".to_owned()];
    let mut all = vec![Tactics::default()];
    for i in 0..SEARCH_SETS {
        labels.push(format!("t{i:03}"));
        all.push(sample_tactics(&mut rng));
    }

    // (勝ち, 負け) の割合
    let evaluate = |tactics: &[Tactics], seeds: std::ops::Range<u64>| -> Vec<(f32, f32)> {
        let n = names.len();
        let mut games = Vec::new();
        for t in 0..tactics.len() {
            for cpu in 0..n {
                for opp in 0..n {
                    for cpu_is_p1 in [true, false] {
                        for seed in seeds.clone() {
                            games.push((t, Game { cpu, opp, opponent: Opponent::Random, cpu_is_p1, seed }));
                        }
                    }
                }
            }
        }
        let outcomes: Vec<Outcome> = parallel_map(&games, |&(t, g)| {
            let setup = Setup { tactics: tactics[t], stage: &stage, exports: &exports, knowledge: &knowledge, terrain: &terrain };
            match opts.engine {
                Engine::Rs => play::<RsPhysics>(&setup, g),
                Engine::Cpp => play::<CppPhysics>(&setup, g),
            }
        });
        (0..tactics.len())
            .map(|t| {
                let v: Vec<&Outcome> = games.iter().zip(&outcomes).filter(|((i, _), _)| *i == t).map(|(_, o)| o).collect();
                let rate = |r: i8| v.iter().filter(|o| o.result == r).count() as f32 / v.len().max(1) as f32;
                (rate(1), rate(-1))
            })
            .collect()
    };

    let stage1 = evaluate(&all, 0..2);
    let mut order: Vec<usize> = (0..all.len()).collect();
    order.sort_by(|&a, &b| stage1[b].0.total_cmp(&stage1[a].0));
    let mut finalists: Vec<usize> = order.iter().copied().take(FINALISTS).collect();
    if !finalists.contains(&0) {
        finalists.push(0);
    }
    let stage2 = evaluate(&finalists.iter().map(|&i| all[i]).collect::<Vec<_>>(), 0..RANDOM_GAMES);
    let pick = (0..finalists.len())
        .filter(|&j| finalists[j] != 0)
        .max_by(|&a, &b| stage2[a].0.total_cmp(&stage2[b].0))
        .unwrap_or(0);
    let chosen = all[finalists[pick]];
    let holdout = evaluate(&[chosen], HOLDOUT_SEED..HOLDOUT_SEED + RANDOM_GAMES)[0];

    let mut md = String::new();
    let _ = writeln!(md, "# E5 行動の数値の探索（{} / {}）\n", opts.stage, opts.engine.name());
    let _ = writeln!(
        md,
        "評価はランダム相手の勝率（全キャラの組み合わせ、P1・P2）。1段目 {} 組 × 196 試合、2段目 上位 {FINALISTS} 組 × 490 試合、検証は別の種で 490 試合。\n",
        all.len()
    );
    let _ = writeln!(md, "| 組 | 1段目 勝ち / 負け | 2段目 勝ち / 負け | 数値 |");
    let _ = writeln!(md, "| --- | --- | --- | --- |");
    for (j, &i) in finalists.iter().enumerate() {
        let _ = writeln!(
            md,
            "| {}{} | {:.1}% / {:.1}% | {:.1}% / {:.1}% | {:?} |",
            labels[i],
            if j == pick { "（選択）" } else { "" },
            stage1[i].0 * 100.0,
            stage1[i].1 * 100.0,
            stage2[j].0 * 100.0,
            stage2[j].1 * 100.0,
            all[i]
        );
    }
    let _ = writeln!(md, "\n選んだ組の検証：勝ち {:.1}% / 負け {:.1}%", holdout.0 * 100.0, holdout.1 * 100.0);

    let out = data_dir.join("results");
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join(format!("e5-{}-tactics.toml", opts.stage)), toml::to_string(&chosen)?)?;
    let path = out.join(format!("e5-search-{}.md", opts.stage));
    std::fs::write(&path, &md)?;
    println!("{md}");
    println!("report: {}", path.display());
    Ok(true)
}

fn sample_tactics(rng: &mut Pcg32) -> Tactics {
    let mut u = |lo: f32, hi: f32| lo + (hi - lo) * rng.unit();
    let low = u(0.2, 1.0);
    let min_hold = u(2.0, 4.99) as u32;
    Tactics {
        dodge_radius: u(0.0, 2.5),
        stomp_dx: u(0.2, 1.0),
        stomp_dy: (low, low + u(0.5, 1.5)),
        min_hold,
        max_hold: min_hold + u(5.0, 40.0) as u32,
        tilt_per_unit: u(2.0, 15.0),
        max_tilt_deg: u(10.0, 40.0),
        travel_dx: u(1.5, 5.0),
        ..Tactics::default()
    }
}
