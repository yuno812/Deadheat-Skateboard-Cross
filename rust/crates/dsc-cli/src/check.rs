//! `dsc check`：書き出しデータと移植が、Unityプロジェクトの今の状態に追いついているかを確かめる。

use std::path::Path;

use dsc_core::export::StageDir;
use dsc_core::source::{self, PORTED_SOURCES, Source, SourceStatus};

/// 問題がなければ true
pub fn run(project_root: &Path, data_dir: &Path, stages: &[String]) -> bool {
    let stages = if stages.is_empty() { exported_stages(data_dir) } else { stages.to_vec() };
    let mut ok = true;

    if stages.is_empty() {
        println!("no exports in {}", data_dir.join("export").display());
        ok = false;
    }
    for stage in &stages {
        println!("export {stage}");
        ok &= check_stage(project_root, &StageDir::new(data_dir, stage));
    }
    if !ok {
        println!("  -> re-export: pwsh scripts/export-sim-data.ps1");
    }

    println!("port");
    let mut port_ok = true;
    for &(path, sha256) in PORTED_SOURCES {
        port_ok &= report(project_root, path, sha256, path);
    }
    if !port_ok {
        println!("  -> review the dsc-core port, then update PORTED_SOURCES in dsc-core/src/source.rs");
    }
    ok && port_ok
}

fn check_stage(project_root: &Path, dir: &StageDir) -> bool {
    let mut ok = match dir.load_stage() {
        Ok(stage) => check_sources(project_root, "stage.json", &stage.sources),
        Err(e) => error(&e),
    };
    match dir.characters() {
        Ok(names) => {
            for name in names {
                ok &= match dir.load_character(&name) {
                    Ok(c) => check_sources(project_root, &format!("{name}.json"), &c.sources),
                    Err(e) => error(&e),
                };
            }
        }
        Err(e) => ok = error(&e),
    }
    ok
}

fn check_sources(project_root: &Path, file: &str, sources: &[Source]) -> bool {
    let mut ok = true;
    for s in sources {
        ok &= report(project_root, &s.path, &s.sha256, &format!("{file}: {}", s.path));
    }
    if ok {
        println!("  ok       {file}");
    }
    ok
}

fn report(project_root: &Path, path: &str, sha256: &str, label: &str) -> bool {
    match source::status(project_root, path, sha256) {
        SourceStatus::Unchanged => true,
        SourceStatus::Changed { current } => {
            println!("  CHANGED  {label} (now {current})");
            false
        }
        SourceStatus::Missing => {
            println!("  MISSING  {label}");
            false
        }
    }
}

fn error(e: &dyn std::error::Error) -> bool {
    println!("  ERROR    {e}");
    false
}

fn exported_stages(data_dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(data_dir.join("export")) else {
        return Vec::new();
    };
    let mut stages: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    stages.sort();
    stages
}
