//! Unityが記録したトレース（data/traces/unity、gitには入れない）を読めるか確かめる。
//! scripts/trace-unity.ps1 を実行していなければ何もしない。

use std::path::Path;

use dsc_scenario::{Scenario, trace};

#[test]
fn unity_traces_parse_and_match_their_scenarios() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let Ok(entries) = std::fs::read_dir(data.join("traces/unity")) else {
        eprintln!("no Unity traces; run scripts/trace-unity.ps1");
        return;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "csv") {
            continue;
        }
        let id = path.file_stem().unwrap().to_str().unwrap().to_owned();
        let rows = trace::parse(&std::fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("{id}: {e}"));

        let scenario_path = data.join("scenarios/R2500").join(format!("{id}.json"));
        let Ok(text) = std::fs::read_to_string(&scenario_path) else {
            continue;
        };
        let scenario = Scenario::from_json(&text).unwrap();
        assert_eq!(rows.len(), scenario.steps as usize * 2, "{id}");
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(row.step as usize, i / 2, "{id}");
            assert_eq!(row.slot as usize, i % 2 + 1, "{id}");
        }
    }
}
