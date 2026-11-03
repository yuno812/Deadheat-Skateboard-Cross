//! `dsc` コマンド：check / gen / run / compare / sweep / bench（各段階で実装）

mod check;
mod generate;
mod paths;

use std::process::ExitCode;

const USAGE: &str = "usage: dsc <check|gen|run|compare|sweep|bench> ...

  check [STAGE...]        書き出しデータと移植が、Unityプロジェクトの今の状態に追いついているか確かめる
  gen verify [STAGE]      照合用シナリオ S1〜S11 を data/scenarios/<STAGE>/ に書き出す（既定 R2500）";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = paths::project_root();
    let data_dir = paths::data_dir(&root);
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["check", stages @ ..] => {
            let stages: Vec<String> = stages.iter().map(|s| s.to_string()).collect();
            if check::run(&root, &data_dir, &stages) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        ["gen", "verify", rest @ ..] if rest.len() <= 1 => {
            match generate::verify(&data_dir, rest.first().copied().unwrap_or("R2500")) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("dsc gen verify: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        [cmd @ ("run" | "compare" | "sweep" | "bench"), ..] => {
            eprintln!("dsc {cmd}: not implemented yet");
            ExitCode::FAILURE
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}
