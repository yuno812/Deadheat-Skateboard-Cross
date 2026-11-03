//! `dsc` コマンド：check / gen / run / compare / sweep / bench（各段階で実装）

mod check;
mod paths;

use std::process::ExitCode;

const USAGE: &str = "usage: dsc <check|gen|run|compare|sweep|bench> ...

  check [STAGE...]   書き出しデータと移植が、Unityプロジェクトの今の状態に追いついているか確かめる";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("check") => {
            let root = paths::project_root();
            if check::run(&root, &paths::data_dir(&root), &args[1..]) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Some(cmd @ ("gen" | "run" | "compare" | "sweep" | "bench")) => {
            eprintln!("dsc {cmd}: not implemented yet");
            ExitCode::FAILURE
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}
