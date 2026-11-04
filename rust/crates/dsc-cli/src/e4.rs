//! E4：ジャンプ予測の精度（仕様書「CPU調整」）。離した瞬間に予測した軌道と、実際に飛んだ軌道を比べる。
//!
//! - 出発状態：各プレイヤーを自分のお椀のランダムな位置に立たせ、ランダムに走らせてから、
//!   ジャンプを 3〜12 ステップ押し、押している最後の数ステップだけランダムに回して傾けて離す（S4 と同じやり方）
//! - 予測：dsc_ai::jump の naive（素朴な放物線）と predict（ゲームの仕組みを入れたもの）
//! - 比べるもの：最高点、着地した瞬間の位置、着地までの時間（地形で予測）、飛んでいる間の最大のずれ

use std::fmt::Write as _;
use std::path::Path;

use dsc_ai::jump::{JumpModel, naive, predict};
use dsc_ai::landing::Wheels;
use dsc_ai::terrain::Terrain;
use dsc_core::export::{CharacterExport, Pose, StageDir, StageExport};
use dsc_core::sim::{CppPhysics, Match, Physics, PlayerInput, RsPhysics};
use dsc_scenario::rng::Pcg32;

use crate::Options;
use crate::e2::observe;
use crate::e3::{terrain, wheels};
use crate::run::{Engine, parallel_map};

const SETTLE: u32 = 50;
const STARTS: u32 = 500;
/// 離してから着地を待つ最大ステップ数
const MAX_FLIGHT: usize = 150;

#[derive(Clone, Copy)]
struct Plan {
    x: f32,
    drive: f32,
    drive_steps: u32,
    press_steps: u32,
    rotate: f32,
    rotate_steps: u32,
}

/// 1回のジャンプの誤差（予測 − 実際）
#[derive(Clone, Copy, Default)]
struct Errors {
    apex: f32,
    touchdown: f32,
    flight_steps: f32,
    max: f32,
}

struct Sample {
    naive: Errors,
    model: Errors,
}

/// 離した瞬間に予測した軌道（naive, model）
type Predictions = (Vec<[f32; 2]>, Vec<[f32; 2]>);

struct Env<'a> {
    stage: &'a StageExport,
    export: &'a CharacterExport,
    terrain: &'a Terrain,
    wheels: Wheels,
    model: JumpModel,
}

pub fn e4(data_dir: &Path, opts: &Options) -> Result<bool, Box<dyn std::error::Error>> {
    let dir = StageDir::new(data_dir, &opts.stage);
    let stage = dir.load_stage()?;
    let names = dir.characters()?;
    let terrain = terrain(&stage);

    let mut md = String::new();
    let _ = writeln!(md, "# E4 ジャンプ予測の精度（{} / {}）\n", opts.stage, opts.engine.name());
    let _ = writeln!(
        md,
        "離した瞬間に予測した軌道と、実際の軌道の差（絶対値の中央値 / 90%点）。naive は素朴な放物線、model はゲームの仕組み（空中でも横の速さに上限、重心まわりの回転、離すステップの減速）を入れた予測。\n"
    );
    let _ = writeln!(md, "| キャラ | ジャンプ数 | 最高点 naive | 最高点 model | 着地位置 naive | 着地位置 model | 着地までのステップ naive | 着地までのステップ model | 最大のずれ naive | 最大のずれ model |");
    let _ = writeln!(md, "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |");

    for (c, name) in names.iter().enumerate() {
        let export = dir.load_character(name)?;
        let slot = export.slot(1).ok_or("slot 1")?;
        let env = Env {
            stage: &stage,
            export: &export,
            terrain: &terrain,
            wheels: wheels(&export),
            model: JumpModel {
                jump_force: export.stats.jump_force,
                max_speed: export.stats.max_speed,
                deceleration: export.stats.deceleration,
                center_of_mass: slot.poses[&Pose::Normal].center_of_mass,
                gravity_y: stage.world.gravity[1],
                dt: stage.world.dt,
            },
        };
        let plans = plans(c as u64, &stage);
        let samples: Vec<Sample> = parallel_map(&plans, |p| match opts.engine {
            Engine::Rs => attempt::<RsPhysics>(&env, *p),
            Engine::Cpp => attempt::<CppPhysics>(&env, *p),
        })
        .into_iter()
        .flatten()
        .flatten()
        .collect();

        let cell = |f: &dyn Fn(&Sample) -> f32| {
            let mut v: Vec<f32> = samples.iter().map(|s| f(s).abs()).filter(|x| x.is_finite()).collect();
            v.sort_by(f32::total_cmp);
            let q = |p: f32| v.get(((v.len() as f32 * p) as usize).min(v.len().saturating_sub(1))).copied().unwrap_or(f32::NAN);
            format!("{:.3} / {:.3}", q(0.5), q(0.9))
        };
        let _ = writeln!(
            md,
            "| {name} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            samples.len(),
            cell(&|s| s.naive.apex),
            cell(&|s| s.model.apex),
            cell(&|s| s.naive.touchdown),
            cell(&|s| s.model.touchdown),
            cell(&|s| s.naive.flight_steps),
            cell(&|s| s.model.flight_steps),
            cell(&|s| s.naive.max),
            cell(&|s| s.model.max)
        );
        println!("{name}: {} jumps; touchdown error naive {} model {}", samples.len(), cell(&|s| s.naive.touchdown), cell(&|s| s.model.touchdown));
    }

    let out = data_dir.join("results");
    std::fs::create_dir_all(&out)?;
    let path = out.join(format!("e4-{}.md", opts.stage));
    std::fs::write(&path, md)?;
    println!("report: {}", path.display());
    Ok(true)
}

/// P1・P2それぞれの計画（自分のお椀のランダムな位置）
fn plans(character: u64, stage: &StageExport) -> Vec<[Plan; 2]> {
    let mut rng = Pcg32::new(6, character);
    let side = stage.spawn.map(|s| s[0].signum());
    (0..STARTS)
        .map(|_| {
            side.map(|side| {
                let press_steps = 3 + rng.below(10);
                Plan {
                    x: side * (3.2 + 2.8 * rng.unit()),
                    drive: rng.below(3) as f32 - 1.0,
                    drive_steps: rng.below(41),
                    press_steps,
                    rotate: if rng.below(2) == 0 { 1.0 } else { -1.0 },
                    rotate_steps: rng.below(press_steps.min(10) + 1),
                }
            })
        })
        .collect()
}

fn attempt<P: Physics>(env: &Env, plan: [Plan; 2]) -> [Option<Sample>; 2] {
    let Ok(mut m) = Match::<P>::new(env.stage, env.export, env.export) else { return [None, None] };
    for (i, p) in plan.iter().enumerate() {
        m.apply_init(i as u8 + 1, Some([p.x, -2.5]), Some(0.0), None, None);
    }
    for _ in 0..SETTLE {
        m.step([PlayerInput::default(); 2]);
    }

    let release = plan.map(|p| p.drive_steps + p.press_steps);
    let end = release.iter().max().copied().unwrap_or(0) as usize + MAX_FLIGHT + 1;
    let mut predicted: [Option<Predictions>; 2] = [None, None];
    let mut before_vy = [0.0f32; 2];
    let mut actual: [Vec<[f32; 2]>; 2] = [Vec::new(), Vec::new()];
    let mut airborne = [false; 2];
    let mut touchdown: [Option<usize>; 2] = [None; 2];

    for step in 0..end as u32 {
        let mut inputs = [PlayerInput::default(); 2];
        for (i, p) in plan.iter().enumerate() {
            let input = &mut inputs[i];
            if step < p.drive_steps + p.press_steps {
                input.move_x = p.drive;
            }
            if step >= p.drive_steps && step < release[i] {
                input.jump = true;
                if step + p.rotate_steps >= release[i] {
                    input.rotate = p.rotate;
                }
            }
            if step == release[i] {
                let me = observe(&m.snapshot(i as u8 + 1));
                before_vy[i] = me.vel.y;
                if me.ground_count > 0 {
                    predicted[i] = Some((naive(&env.model, &me, MAX_FLIGHT), predict(&env.model, &me, MAX_FLIGHT)));
                }
            }
        }
        m.step(inputs);
        for i in 0..2 {
            if predicted[i].is_none() || step < release[i] || touchdown[i].is_some() {
                continue;
            }
            let s = m.snapshot(i as u8 + 1);
            if step == release[i] && s.vel[1] - before_vy[i] < 3.0 {
                // ジャンプしなかった（離したとき接地していなかった）
                predicted[i] = None;
                continue;
            }
            actual[i].push(s.pos);
            let touching = s.ground > 0 || s.body_ground > 0 || s.deck_ground > 0;
            if !touching {
                airborne[i] = true;
            } else if airborne[i] {
                touchdown[i] = Some(actual[i].len() - 1);
            }
            if actual[i].len() >= MAX_FLIGHT {
                predicted[i] = None;
            }
        }
    }

    [0, 1].map(|i| {
        let (n, p) = predicted[i].as_ref()?;
        let land = touchdown[i]?;
        let a = &actual[i][..=land];
        Some(Sample { naive: errors(env, n, a, land), model: errors(env, p, a, land) })
    })
}

fn errors(env: &Env, predicted: &[[f32; 2]], actual: &[[f32; 2]], land: usize) -> Errors {
    let dist = |a: [f32; 2], b: [f32; 2]| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt();
    let apex = |v: &[[f32; 2]]| v.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
    // 予測の着地：車輪の下端が真下の床に届いたステップ（まっすぐ立っているとして）
    let predicted_land = predicted
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, p)| env.terrain.floor_below(p[0], p[1]).is_none_or(|(floor, _)| p[1] - env.wheels.ride_height <= floor))
        .map_or(f32::NAN, |(k, _)| k as f32);
    Errors {
        apex: apex(&predicted[..=land]) - apex(actual),
        touchdown: dist(predicted[land], actual[land]),
        flight_steps: predicted_land - land as f32,
        max: (0..=land).map(|k| dist(predicted[k], actual[k])).fold(0.0, f32::max),
    }
}
