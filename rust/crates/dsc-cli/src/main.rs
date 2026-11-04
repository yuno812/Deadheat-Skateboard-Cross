//! `dsc` コマンド：check / gen / run / compare / sweep / bench（各段階で実装）

mod bench;
mod check;
mod compare;
mod distribution;
mod e2;
mod exp;
mod generate;
mod paths;
mod run;

use std::process::ExitCode;

use regex::Regex;

const USAGE: &str = "usage: dsc <check|gen|run|compare|bench|exp|sweep> ...

  check [STAGE...]                     書き出しデータと移植が、Unityプロジェクトの今の状態に追いついているか確かめる
  gen verify [STAGE]                   照合用シナリオ S1〜S11 を data/scenarios/<STAGE>/ に書き出す
  gen random [STAGE] [--count N] [--seed S]  ランダム照合 R1 のシナリオを書き出す（既定 100 本/キャラ、種 1）
  run [OPTIONS]        シナリオを再生して data/traces/sim-<ENGINE>/ にトレースを書く
  compare [OPTIONS]    Unity とシミュレーターのトレースを照合する（data/results/compare-<STAGE>-<ENGINE>.md）
  bench [OPTIONS]      シナリオの再生速度を測る
  exp e1 [OPTIONS]     E1：回転だけで起き上がれるか（data/results/e1-<STAGE>.md と .csv）
  exp e2 [OPTIONS]     E2：復帰処理の数値探索（data/results/e2-<STAGE>.md、選んだ値は e2-<STAGE>-best.toml）
  exp e2-check CHARACTER [OPTIONS]  選んだ値で失敗したサンプルを調べる（軌跡は data/traces/e2-fail/）

  OPTIONS: [STAGE] [--filter REGEX] [--engine rs|cpp]
    STAGE の既定は R2500。REGEX はシナリオ id の正規表現。ENGINE の既定は cpp（rs は照合の突き合わせ用）";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = paths::project_root();
    let data_dir = paths::data_dir(&root);
    let result: Result<bool, Box<dyn std::error::Error>> = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["check", stages @ ..] => {
            let stages: Vec<String> = stages.iter().map(|s| s.to_string()).collect();
            Ok(check::run(&root, &data_dir, &stages))
        }
        ["gen", "verify", rest @ ..] if rest.len() <= 1 => {
            generate::verify(&data_dir, rest.first().copied().unwrap_or("R2500")).map(|()| true)
        }
        ["gen", "random", rest @ ..] => gen_random(&data_dir, rest),
        [cmd @ ("run" | "compare" | "bench"), rest @ ..] => match (Options::parse(rest), *cmd) {
            (Ok(opts), "run") => run::run(&data_dir, &opts).map(|()| true),
            (Ok(opts), "compare") => compare::compare(&data_dir, &opts),
            (Ok(opts), _) => bench::bench(&data_dir, &opts).map(|()| true),
            (Err(e), _) => Err(e),
        },
        ["exp", "e1", rest @ ..] => Options::parse(rest).and_then(|opts| exp::e1(&data_dir, &opts)),
        ["exp", "e2", rest @ ..] => Options::parse(rest).and_then(|opts| e2::e2(&data_dir, &opts)),
        ["exp", "e2-check", character, rest @ ..] => Options::parse(rest).and_then(|opts| e2::check(&data_dir, &opts, character)),
        ["sweep", ..] => Err("dsc sweep: not implemented yet".into()),
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("dsc: {e}");
            ExitCode::FAILURE
        }
    }
}

fn gen_random(data_dir: &std::path::Path, args: &[&str]) -> Result<bool, Box<dyn std::error::Error>> {
    let (mut stage, mut count, mut seed) = ("R2500", 100u32, 1u64);
    let mut it = args.iter();
    while let Some(&arg) = it.next() {
        match arg {
            "--count" => count = it.next().ok_or("--count needs a value")?.parse()?,
            "--seed" => seed = it.next().ok_or("--seed needs a value")?.parse()?,
            other if !other.starts_with('-') => stage = other,
            other => return Err(format!("unknown option {other}").into()),
        }
    }
    generate::random(data_dir, stage, count, seed).map(|()| true)
}

pub struct Options {
    pub stage: String,
    pub filter: Option<Regex>,
    pub engine: run::Engine,
}

impl Options {
    fn parse(args: &[&str]) -> Result<Self, Box<dyn std::error::Error>> {
        let mut opts = Options { stage: "R2500".to_owned(), filter: None, engine: run::Engine::Cpp };
        let mut it = args.iter();
        while let Some(&arg) = it.next() {
            match arg {
                "--filter" => opts.filter = Some(Regex::new(it.next().ok_or("--filter needs a value")?)?),
                "--engine" => {
                    let name = it.next().ok_or("--engine needs a value")?;
                    opts.engine = run::Engine::parse(name).ok_or_else(|| format!("unknown engine {name} (rs|cpp)"))?;
                }
                other if !other.starts_with('-') => opts.stage = other.to_owned(),
                other => return Err(format!("unknown option {other}").into()),
            }
        }
        Ok(opts)
    }
}
