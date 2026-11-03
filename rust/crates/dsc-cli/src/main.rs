//! `dsc` コマンド：gen / run / compare / sweep / bench（各段階で実装）

use std::process::ExitCode;

const USAGE: &str = "usage: dsc <gen|run|compare|sweep|bench> ...";

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
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
