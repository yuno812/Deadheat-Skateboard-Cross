//! `dsc gen verify [STAGE]`：照合用シナリオ S1〜S11 を data/scenarios/<STAGE>/ に書き出す。

use std::path::Path;

use dsc_core::export::StageDir;
use dsc_scenario::verify;

pub fn verify(data_dir: &Path, stage: &str) -> Result<(), Box<dyn std::error::Error>> {
    let dir = StageDir::new(data_dir, stage);
    let export = dir.load_stage()?;
    let scenarios = verify::scenarios(&export, &dir.characters()?);

    let out = data_dir.join("scenarios").join(stage);
    std::fs::create_dir_all(&out)?;
    // 前回生成した照合用シナリオ（S<番号>-…）だけを消してから書き直す
    for entry in std::fs::read_dir(&out)? {
        let path = entry?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if is_verify_file(name) {
            std::fs::remove_file(&path)?;
        }
    }
    for s in &scenarios {
        s.validate()?;
        std::fs::write(out.join(format!("{}.json", s.id)), s.to_json())?;
    }
    println!("wrote {} scenarios to {}", scenarios.len(), out.display());
    Ok(())
}

fn is_verify_file(name: &str) -> bool {
    let Some(rest) = name.strip_prefix('S').and_then(|r| r.strip_suffix(".json")) else {
        return false;
    };
    let digits = rest.chars().take_while(char::is_ascii_digit).count();
    digits > 0 && rest[digits..].starts_with('-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_verify_scenarios_are_replaced() {
        assert!(is_verify_file("S1-zero.json"));
        assert!(is_verify_file("S10-okuda.json"));
        assert!(!is_verify_file("R1-zero-0001.json"));
        assert!(!is_verify_file("Sample.json"));
        assert!(!is_verify_file("S1-zero.csv"));
    }
}
