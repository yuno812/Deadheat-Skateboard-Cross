//! CPU 調整の実験（仕様書「CPU調整」）。結果は data/results/<実験>-<STAGE>.csv と .md に書く。
//!
//! E1：回転だけで起き上がれるか。各キャラを 90〜180°（左右とも、5°刻み）に傾けて地上に置き、
//! 落ち着くまで待ってから、左回転・右回転それぞれで次の2つを試す。
//! - A 押し続ける（5秒）：真上付近（|角度| < 20°）を通るか。押し続けると最大 900 度/秒で回り続けるため、
//!   両輪が着く前に通り過ぎる。回転で起き上がれる向きまで持っていけるかを見る
//! - B 真上付近（|角度| < 30°）で離す：離してから2秒以内に両輪が着くか。一番単純な復帰の制御

use std::fmt::Write as _;
use std::path::Path;

use dsc_core::export::{CharacterExport, StageDir, StageExport};
use dsc_core::sim::{CppPhysics, Match, Physics, PlayerInput, RsPhysics};

use crate::Options;
use crate::run::{Engine, parallel_map};

/// 置いてから落ち着くまで待つステップ数（照合用シナリオと同じ）
const SETTLE: u32 = 50;
/// 回転を押し続ける最大ステップ数（5秒）
const ROTATE: u32 = 250;
/// B で離してから両輪が着くのを待つステップ数（2秒）
const AFTER_RELEASE: u32 = 100;
/// A で「真上付近を通った」とみなす角度
const PASS_ANGLE: f32 = 20.0;
/// B で回転を離す角度
const RELEASE_ANGLE: f32 = 30.0;
const DT: f32 = 0.02;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dir {
    Left,
    Right,
}

impl Dir {
    fn input(self) -> f32 {
        match self {
            Dir::Left => 1.0,
            Dir::Right => -1.0,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Dir::Left => "L",
            Dir::Right => "R",
        }
    }
}

struct Case {
    character: usize,
    angle: i32,
    dir: Dir,
}

/// 1プレイヤーぶんの結果
#[derive(Debug, Clone, Copy, Default)]
struct Outcome {
    /// 回し始める直前の角度
    settled_angle: f32,
    /// 回し始める前から両輪が着いていた
    upright_before: bool,
    /// A：回し始めてから真上付近を通るまでのステップ数
    pass_steps: Option<u32>,
    /// B：回し始めてから両輪が着くまでのステップ数（真上付近で離した場合）
    release_steps: Option<u32>,
}

pub fn e1(data_dir: &Path, opts: &Options) -> Result<bool, Box<dyn std::error::Error>> {
    let dir = StageDir::new(data_dir, &opts.stage);
    let stage = dir.load_stage()?;
    let names = dir.characters()?;
    let characters: Vec<CharacterExport> = names.iter().map(|n| dir.load_character(n)).collect::<Result<_, _>>()?;

    let angles: Vec<i32> = (-175..=-90).step_by(5).chain((90..=180).step_by(5)).collect();
    let cases: Vec<Case> = (0..names.len())
        .flat_map(|c| angles.iter().flat_map(move |&angle| [Dir::Left, Dir::Right].map(|dir| Case { character: c, angle, dir })))
        .collect();

    let outcomes = parallel_map(&cases, |case| {
        let export = &characters[case.character];
        match opts.engine {
            Engine::Rs => run_case::<RsPhysics>(&stage, export, case),
            Engine::Cpp => run_case::<CppPhysics>(&stage, export, case),
        }
    });

    let mut csv = String::from("character,slot,angle,dir,settled_angle,upright_before,pass_steps,release_steps\n");
    let mut rows = Vec::new();
    for (case, result) in cases.iter().zip(outcomes) {
        for (slot, o) in [1u8, 2].into_iter().zip(result?) {
            let opt = |v: Option<u32>| v.map_or(String::new(), |s| s.to_string());
            let _ = writeln!(
                csv,
                "{},{slot},{},{},{:.1},{},{},{}",
                names[case.character],
                case.angle,
                case.dir.name(),
                o.settled_angle,
                o.upright_before,
                opt(o.pass_steps),
                opt(o.release_steps)
            );
            rows.push(Row { case, slot, o });
        }
    }

    let out = data_dir.join("results");
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join(format!("e1-{}.csv", opts.stage)), csv)?;
    let path = out.join(format!("e1-{}.md", opts.stage));
    std::fs::write(&path, render(&opts.stage, opts.engine.name(), &names, &angles, &rows))?;
    println!("{}", summary_table(&names, &rows));
    println!("report: {}", path.display());
    Ok(true)
}

struct Row<'a> {
    case: &'a Case,
    slot: u8,
    o: Outcome,
}

/// P1とP2に同じキャラを同じ角度で置き、同じ向きに回す（照合用シナリオと同じく、反転なし・ありを1回で取る）
fn run_case<P: Physics>(stage: &StageExport, export: &CharacterExport, case: &Case) -> Result<[Outcome; 2], String> {
    let new = || -> Result<Match<P>, String> {
        let mut m = Match::<P>::new(stage, export, export).map_err(|e| e.to_string())?;
        for slot in [1, 2] {
            m.apply_init(slot, None, Some(case.angle as f32), None, None);
        }
        for _ in 0..SETTLE {
            m.step([PlayerInput::default(); 2]);
        }
        Ok(m)
    };
    let rotate = PlayerInput { rotate: case.dir.input(), ..Default::default() };

    // A：押し続ける
    let mut m = new()?;
    let mut out = [1, 2].map(|slot| {
        let s = m.snapshot(slot);
        Outcome { settled_angle: s.angle_deg, upright_before: s.ground >= 2, ..Default::default() }
    });
    for step in 0..ROTATE {
        m.step([rotate; 2]);
        for (i, o) in out.iter_mut().enumerate() {
            if o.pass_steps.is_none() && m.snapshot(i as u8 + 1).angle_deg.abs() < PASS_ANGLE {
                o.pass_steps = Some(step);
            }
        }
    }

    // B：真上付近で離し、離してから2秒以内に両輪が着くか
    let mut m = new()?;
    let mut released: [Option<u32>; 2] = [None; 2];
    for step in 0..ROTATE + AFTER_RELEASE {
        let mut inputs = [PlayerInput::default(); 2];
        for i in 0..2 {
            let s = m.snapshot(i as u8 + 1);
            if released[i].is_none() && step < ROTATE && s.angle_deg.abs() < RELEASE_ANGLE {
                released[i] = Some(step);
            }
            if released[i].is_none() && step < ROTATE {
                inputs[i] = rotate;
            }
        }
        m.step(inputs);
        for (i, o) in out.iter_mut().enumerate() {
            let within = released[i].is_some_and(|r| step < r + AFTER_RELEASE);
            if o.release_steps.is_none() && within && m.snapshot(i as u8 + 1).ground >= 2 {
                o.release_steps = Some(step);
            }
        }
    }
    Ok(out)
}

fn median(mut v: Vec<u32>) -> Option<f32> {
    if v.is_empty() {
        return None;
    }
    v.sort_unstable();
    let n = v.len();
    Some(if n % 2 == 1 { v[n / 2] as f32 } else { (v[n / 2 - 1] + v[n / 2]) as f32 / 2.0 })
}

/// キャラ × 方向ごとの成功率と、時間の中央値（秒）。回す前から立っていたものは除く
fn summary_table(names: &[String], rows: &[Row]) -> String {
    let mut md = String::new();
    let _ = writeln!(md, "| キャラ | A 真上を通る 左 / 右 | A 時間の中央値 左 / 右 | B 離して接地 左 / 右 | B 時間の中央値 左 / 右 | 回す前から立っていた |");
    let _ = writeln!(md, "| --- | --- | --- | --- | --- | --- |");
    for (c, name) in names.iter().enumerate() {
        let mine: Vec<&Row> = rows.iter().filter(|r| r.case.character == c).collect();
        let stat = |d: Dir, f: &dyn Fn(&Outcome) -> Option<u32>| {
            let v: Vec<&&Row> = mine.iter().filter(|r| r.case.dir == d && !r.o.upright_before).collect();
            let ok: Vec<u32> = v.iter().filter_map(|r| f(&r.o)).collect();
            let rate = format!("{:.0}%", ok.len() as f32 / v.len().max(1) as f32 * 100.0);
            (rate, median(ok).map_or("-".to_owned(), |s| format!("{:.2}秒", s * DT)))
        };
        let pass = |o: &Outcome| o.pass_steps;
        let release = |o: &Outcome| o.release_steps;
        let ((al, alt), (ar, art)) = (stat(Dir::Left, &pass), stat(Dir::Right, &pass));
        let ((bl, blt), (br, brt)) = (stat(Dir::Left, &release), stat(Dir::Right, &release));
        let already = mine.iter().filter(|r| r.o.upright_before).count();
        let _ = writeln!(md, "| {name} | {al} / {ar} | {alt} / {art} | {bl} / {br} | {blt} / {brt} | {already}/{} |", mine.len());
    }
    md
}

fn render(stage: &str, engine: &str, names: &[String], angles: &[i32], rows: &[Row]) -> String {
    let mut md = String::new();
    let _ = writeln!(md, "# E1 回転だけで起き上がれるか（{stage} / {engine}）\n");
    let _ = writeln!(
        md,
        "各キャラを ±90〜180° に傾けて置き、{SETTLE} ステップ待ってから回転を押す。\n\n\
         - A：{} 秒押し続け、真上付近（|角度| < {PASS_ANGLE}°）を通るか\n\
         - B：|角度| < {RELEASE_ANGLE}° になったら離し、{} 秒以内に両輪が地面に着くか\n",
        ROTATE as f32 * DT,
        AFTER_RELEASE as f32 * DT
    );
    md.push_str(&summary_table(names, rows));
    let _ = writeln!(md, "\n## 角度ごとの結果\n");
    let _ = writeln!(md, "「A / B」の秒数（回し始めてから）。× は起きなかった、「立」は回す前から立っていた。角度は置いたときの値（反時計回りが正）。\n");
    for (c, name) in names.iter().enumerate() {
        let _ = writeln!(md, "### {name}\n");
        let _ = writeln!(md, "| 角度 | P1 左 | P1 右 | P2 左 | P2 右 |");
        let _ = writeln!(md, "| --- | --- | --- | --- | --- |");
        for &a in angles {
            let cell = |slot: u8, d: Dir| {
                rows.iter()
                    .find(|r| r.case.character == c && r.case.angle == a && r.case.dir == d && r.slot == slot)
                    .map_or("?".to_owned(), |r| {
                        let t = |v: Option<u32>| v.map_or("×".to_owned(), |s| format!("{:.2}", s as f32 * DT));
                        if r.o.upright_before { "立".to_owned() } else { format!("{} / {}", t(r.o.pass_steps), t(r.o.release_steps)) }
                    })
            };
            let _ = writeln!(md, "| {a} | {} | {} | {} | {} |", cell(1, Dir::Left), cell(1, Dir::Right), cell(2, Dir::Left), cell(2, Dir::Right));
        }
        md.push('\n');
    }
    md
}
