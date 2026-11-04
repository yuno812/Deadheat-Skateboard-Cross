//! E6：難易度（仕様書「CPU調整」）。調整済みの CPU（ゲームと同じ rust/profiles/tuned-<STAGE>.toml）に
//! ハンデ（dsc_ai::handicap）を足した 3 つの難易度を戦わせる。
//!
//! 仕様書の目標（上の難易度が下に 65〜80% で勝つ）は、シミュレーターでは届かなかった（ハンデの範囲の上限でも 58%。
//! つよい でもランダム相手に 75% しか勝てず、試合の運の要素が大きい）。そこで難易度の値は遊んで決め
//! （rust/profiles/normal.toml・weak.toml を手で書く）、`exp e6` はその値での勝率を測るだけにする。
//! `exp e6-search` は、目標に向けてハンデの量を探して toml を書き直す（参考用）。
//!
//! - つよい：ハンデなし
//! - e6-search：つよい が ふつう に TARGET の割合で勝つハンデの量を二分探索で求め、ふつう 対 よわい も同じように求める
//! - ハンデの量 h は、反応の遅れ・狙いのブレ・操作のもたつきを同じ割合で強める 1 つの数（level）
//! - 勝率は（勝ち + 引き分け × 0.5）÷ 試合数。キャラの組み合わせ全部 × P1/P2 × 種 SEEDS
//! - シミュレーターはウルトを再現しないので、ウルトなしの勝率。ゲームでは押し損ね（もたつき）や遅れがウルトにも効く
//!
//! 結果は data/results/e6-<STAGE>.md（e6-search は e6-search-<STAGE>.md）に出す。

use std::fmt::Write as _;
use std::path::Path;

use dsc_ai::handicap::Handicapped;
use dsc_ai::rng::Pcg32;
use dsc_ai::terrain::Terrain;
use dsc_ai::tuned::{Stats, brain_for};
use dsc_ai::{BodyObs, Difficulty, Input, Observation, Policy, Profile, Vec2};
use dsc_core::export::{CharacterExport, StageDir, StageExport};
use dsc_core::sim::{CppPhysics, Match, PlayerInput};

use crate::Options;
use crate::e2::observe;
use crate::e3::terrain;
use crate::run::parallel_map;

const MAX_STEPS: u32 = 3000;
/// 探索中の 1 回の評価の種の数と、最後の確認の種の数
const SEEDS: u64 = 3;
const FINAL_SEEDS: u64 = 6;
/// 上の難易度が下に勝つ割合の目標（仕様書の 65〜80% の真ん中）
const TARGET: f32 = 0.725;
const ITERATIONS: usize = 7;
/// 探す範囲（ハンデの量）
const LEVEL_MAX: f32 = 6.0;
/// ハンデの量 1 あたりの、反応の遅れ（ステップ）・狙いのブレ・もたつきの確率
const REACTION_PER_LEVEL: f32 = 6.0;
const AIM_PER_LEVEL: f32 = 0.4;
const SLIP_PER_LEVEL: f32 = 0.06;
const SLIP_MAX: f32 = 0.6;

pub fn level(h: f32) -> Profile {
    Profile {
        reaction_delay_steps: (h * REACTION_PER_LEVEL).round() as u32,
        aim_noise: h * AIM_PER_LEVEL,
        slip: (h * SLIP_PER_LEVEL).min(SLIP_MAX),
    }
}

#[derive(Clone, Copy)]
enum Side {
    Cpu(Profile),
    Random,
}

struct Setup<'a> {
    stage: &'a StageExport,
    exports: &'a [CharacterExport],
    names: &'a [String],
    terrain: &'a Terrain,
}

#[derive(Clone, Copy)]
struct Game {
    a: usize,
    b: usize,
    a_is_p1: bool,
    seed: u64,
}

/// R1・E5 と同じランダムな相手：10〜50 ステップごとに、前後・回転を -1/0/1、ジャンプの押しっぱなしを 30% で選び直す
struct RandomPlayer {
    rng: Pcg32,
    input: Input,
    left: u32,
}

impl Policy for RandomPlayer {
    fn act(&mut self, _obs: &Observation) -> Input {
        if self.left == 0 {
            self.left = 10 + self.rng.below(41);
            self.input = Input {
                move_x: self.rng.below(3) as f32 - 1.0,
                rotate: self.rng.below(3) as f32 - 1.0,
                jump: self.rng.below(10) < 3,
                ult: false,
            };
        }
        self.left -= 1;
        self.input
    }
}

fn agent(setup: &Setup, side: Side, character: usize, seed: u64) -> Box<dyn Policy> {
    match side {
        Side::Cpu(profile) => {
            let e = &setup.exports[character];
            let stats = Stats {
                jump_force: e.stats.jump_force,
                max_speed: e.stats.max_speed,
                deceleration: e.stats.deceleration,
                gravity_y: setup.stage.world.gravity[1],
                dt: setup.stage.world.dt,
            };
            Box::new(Handicapped::new(brain_for(&setup.names[character], stats, setup.terrain.clone()), profile, seed))
        }
        Side::Random => Box::new(RandomPlayer { rng: Pcg32::new(7, seed), input: Input::default(), left: 0 }),
    }
}

/// 1試合。A から見て 1 = 勝ち、0.5 = 引き分け、0 = 負け
fn play(setup: &Setup, a: Side, b: Side, g: Game) -> f32 {
    let (p1, p2) = if g.a_is_p1 { (g.a, g.b) } else { (g.b, g.a) };
    let Ok(mut m) = Match::<CppPhysics>::new(setup.stage, &setup.exports[p1], &setup.exports[p2]) else { return 0.5 };
    let agent_a = agent(setup, a, g.a, g.seed * 2);
    let agent_b = agent(setup, b, g.b, g.seed * 2 + 1);
    let mut agents = if g.a_is_p1 { [agent_a, agent_b] } else { [agent_b, agent_a] };
    let bounds = Vec2 { x: setup.stage.out_of_area[0], y: setup.stage.out_of_area[1] };
    for step in 0..MAX_STEPS {
        let bodies: [BodyObs; 2] = [observe(&m.snapshot(1)), observe(&m.snapshot(2))];
        let mut inputs = [PlayerInput::default(); 2];
        for i in 0..2 {
            let obs = Observation { step, me: bodies[i], opp: bodies[1 - i], gravity_up: Vec2 { x: 0.0, y: 1.0 }, bounds };
            let input = agents[i].act(&obs);
            inputs[i] = PlayerInput { move_x: input.move_x, rotate: input.rotate, jump: input.jump };
        }
        m.step(inputs);
        if m.snapshot(1).hp <= 0.0 || m.snapshot(2).hp <= 0.0 {
            break;
        }
    }
    let hp = [m.snapshot(1).hp, m.snapshot(2).hp];
    let (a_slot, b_slot) = if g.a_is_p1 { (0, 1) } else { (1, 0) };
    match (hp[a_slot] <= 0.0, hp[b_slot] <= 0.0) {
        (false, true) => 1.0,
        (true, false) => 0.0,
        _ => 0.5,
    }
}

/// A が B に勝つ割合（キャラの組み合わせ全部 × P1/P2 × 種）
fn rate(setup: &Setup, a: Side, b: Side, seeds: u64) -> f32 {
    let n = setup.names.len();
    let mut games = Vec::new();
    for ca in 0..n {
        for cb in 0..n {
            for a_is_p1 in [true, false] {
                for seed in 0..seeds {
                    games.push(Game { a: ca, b: cb, a_is_p1, seed: seed + 1 });
                }
            }
        }
    }
    let scores = parallel_map(&games, |g| play(setup, a, b, *g));
    scores.iter().sum::<f32>() / scores.len().max(1) as f32
}

/// upper が lower(h) に TARGET の割合で勝つハンデの量 h を、[lo, hi] で二分探索する
fn search(setup: &Setup, upper: Profile, lo: f32, hi: f32, md: &mut String, label: &str) -> f32 {
    let (mut lo, mut hi) = (lo, hi);
    let _ = writeln!(md, "\n## {label}\n\n| ハンデの量 | 反応の遅れ（ステップ） | 狙いのブレ | もたつき | 上が勝つ割合 |\n| --- | --- | --- | --- | --- |");
    let row = |h: f32, r: f32, md: &mut String| {
        let p = level(h);
        let _ = writeln!(md, "| {h:.3} | {} | {:.2} | {:.3} | {:.1}% |", p.reaction_delay_steps, p.aim_noise, p.slip, r * 100.0);
        println!("{label}: level {h:.3} -> {:.1}%", r * 100.0);
    };
    let at_hi = rate(setup, Side::Cpu(upper), Side::Cpu(level(hi)), SEEDS);
    row(hi, at_hi, md);
    if at_hi < TARGET {
        let _ = writeln!(md, "\n探す範囲の上限でも目標に届かなかったので、上限の値を使う。");
        return hi;
    }
    for _ in 0..ITERATIONS {
        let mid = (lo + hi) / 2.0;
        let r = rate(setup, Side::Cpu(upper), Side::Cpu(level(mid)), SEEDS);
        row(mid, r, md);
        if r < TARGET {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

/// ステージ、キャラ名、キャラの書き出し
type Loaded = (StageExport, Vec<String>, Vec<CharacterExport>);

fn load(data_dir: &Path, opts: &Options) -> Result<Loaded, Box<dyn std::error::Error>> {
    let dir = StageDir::new(data_dir, &opts.stage);
    let stage = dir.load_stage()?;
    let names = dir.characters()?;
    let exports: Vec<CharacterExport> = names.iter().map(|n| dir.load_character(n)).collect::<Result<_, _>>()?;
    Ok((stage, names, exports))
}

/// 3 つの難易度の値と、組み合わせごとの勝率（種 FINAL_SEEDS）を表にする
fn check_levels(setup: &Setup, levels: [(&str, Profile); 3], md: &mut String) {
    let [(_, strong), (_, normal), (_, weak)] = levels;
    let _ = writeln!(md, "
## 値

| 難易度 | 反応の遅れ（ステップ） | 狙いのブレ | もたつき |
| --- | --- | --- | --- |");
    for (label, p) in levels {
        let _ = writeln!(md, "| {label} | {} | {:.2} | {:.3} |", p.reaction_delay_steps, p.aim_noise, p.slip);
    }
    let checks = [
        ("つよい 対 ふつう", Side::Cpu(strong), Side::Cpu(normal)),
        ("ふつう 対 よわい", Side::Cpu(normal), Side::Cpu(weak)),
        ("つよい 対 よわい", Side::Cpu(strong), Side::Cpu(weak)),
        ("つよい 対 ランダム", Side::Cpu(strong), Side::Random),
        ("ふつう 対 ランダム", Side::Cpu(normal), Side::Random),
        ("よわい 対 ランダム", Side::Cpu(weak), Side::Random),
    ];
    let games = setup.names.len() * setup.names.len() * 2 * FINAL_SEEDS as usize;
    let _ = writeln!(md, "
## 勝率（{games} 試合ずつ、ウルトなし）

| 組み合わせ | 前が勝つ割合 |
| --- | --- |");
    for (label, a, b) in checks {
        let r = rate(setup, a, b, FINAL_SEEDS);
        let _ = writeln!(md, "| {label} | {:.1}% |", r * 100.0);
        println!("{label}: {:.1}%", r * 100.0);
    }
}

fn write_report(data_dir: &Path, name: &str, md: &str) -> Result<(), Box<dyn std::error::Error>> {
    let out = data_dir.join("results");
    std::fs::create_dir_all(&out)?;
    let path = out.join(name);
    std::fs::write(&path, md)?;
    println!("report: {}", path.display());
    Ok(())
}

/// `dsc exp e6`：DLL に埋め込む今の値（rust/profiles/*.toml）で、難易度どうしとランダム相手の勝率を測る
pub fn e6(data_dir: &Path, opts: &Options) -> Result<bool, Box<dyn std::error::Error>> {
    let (stage, names, exports) = load(data_dir, opts)?;
    let terrain = terrain(&stage);
    let setup = Setup { stage: &stage, exports: &exports, names: &names, terrain: &terrain };
    let mut md = String::new();
    let _ = writeln!(md, "# E6 難易度：今の値の勝率（{} / cpp）", opts.stage);
    check_levels(
        &setup,
        [
            ("つよい", Profile::builtin(Difficulty::Strong)),
            ("ふつう", Profile::builtin(Difficulty::Normal)),
            ("よわい", Profile::builtin(Difficulty::Weak)),
        ],
        &mut md,
    );
    write_report(data_dir, &format!("e6-{}.md", opts.stage), &md)?;
    Ok(true)
}

/// `dsc exp e6-search`：上の難易度が下に TARGET の割合で勝つハンデの量を探し、rust/profiles/normal.toml・weak.toml を
/// 書き直す（遊んで決めた値も上書きする）。R2500 では範囲の上限でも届かなかった（シミュレーターの試合は運の要素が大きい）
pub fn search_profiles(project_root: &Path, data_dir: &Path, opts: &Options) -> Result<bool, Box<dyn std::error::Error>> {
    let (stage, names, exports) = load(data_dir, opts)?;
    let terrain = terrain(&stage);
    let setup = Setup { stage: &stage, exports: &exports, names: &names, terrain: &terrain };

    let mut md = String::new();
    let _ = writeln!(md, "# E6 難易度：ハンデの探索（{} / cpp）
", opts.stage);
    let _ = writeln!(
        md,
        "目標：上の難易度が下に {:.1}% で勝つ（仕様書は 65〜80%）。ハンデの量 1 あたり、反応の遅れ {REACTION_PER_LEVEL} ステップ・         狙いのブレ {AIM_PER_LEVEL}・もたつき {SLIP_PER_LEVEL}。探索中は {} 試合で測る（ウルトなし）。",
        TARGET * 100.0,
        names.len() * names.len() * 2 * SEEDS as usize,
    );

    let strong = Profile::default();
    let h_normal = search(&setup, strong, 0.0, LEVEL_MAX, &mut md, "ふつう：つよい 対 ふつう");
    let normal = level(h_normal);
    let h_weak = search(&setup, normal, h_normal, h_normal + LEVEL_MAX, &mut md, "よわい：ふつう 対 よわい");
    let weak = level(h_weak);
    check_levels(&setup, [("つよい", strong), ("ふつう", normal), ("よわい", weak)], &mut md);

    let profiles = project_root.join("rust").join("profiles");
    for (file, label, h, p) in [("normal.toml", "ふつう", h_normal, normal), ("weak.toml", "よわい", h_weak, weak)] {
        let text = format!("# {label}（dsc exp e6-search {} が書いた。ハンデの量 {h:.3}）
{}", opts.stage, toml::to_string(&p)?);
        std::fs::write(profiles.join(file), text)?;
    }
    write_report(data_dir, &format!("e6-search-{}.md", opts.stage), &md)?;
    println!("profiles: {}", profiles.display());
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_grows_every_handicap() {
        assert_eq!(level(0.0), Profile::default());
        let (a, b) = (level(1.0), level(2.0));
        assert!(b.reaction_delay_steps > a.reaction_delay_steps && b.aim_noise > a.aim_noise && b.slip > a.slip);
        assert!(level(100.0).slip <= SLIP_MAX);
    }
}
