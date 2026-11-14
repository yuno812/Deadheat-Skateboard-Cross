//! `dsc tune-profile [STAGE]`：E2・E3・E5 の結果と、書き出しから求めたキャラの形の値をまとめて
//! rust/profiles/tuned-<STAGE>.toml に書く。dsc-ai がこれを DLL に埋め込み、ゲームの CPU が使う。

use std::collections::BTreeMap;
use std::path::Path;

use dsc_ai::brain::Tactics;
use dsc_ai::landing::LandingParams;
use dsc_ai::recovery::RecoveryParams;
use dsc_ai::tuned::{CharacterTuning, Tuned};
use dsc_core::export::{Pose, StageDir};

use crate::e3::wheels;

pub fn run(project_root: &Path, data_dir: &Path, stage: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let results = data_dir.join("results");
    let read = |name: String| std::fs::read_to_string(results.join(&name)).map_err(|e| format!("{name}: {e}（先に dsc exp を実行する）"));
    let recovery: BTreeMap<String, RecoveryParams> = toml::from_str(&read(format!("e2-{stage}-best.toml"))?)?;
    let landing: BTreeMap<String, LandingParams> = toml::from_str(&read(format!("e3-{stage}-best.toml"))?)?;
    let tactics: Tactics = toml::from_str(&read(format!("e5-{stage}-tactics.toml"))?)?;

    let dir = StageDir::new(data_dir, stage);
    let mut characters = BTreeMap::new();
    for name in dir.characters()? {
        let export = dir.load_character(&name)?;
        let slot = export.slot(1).ok_or("slot 1")?;
        characters.insert(
            name.clone(),
            CharacterTuning {
                ride_height: wheels(&export).ride_height,
                center_of_mass: slot.poses[&Pose::Normal].center_of_mass,
                recovery: recovery.get(&name).copied().unwrap_or_default(),
                landing: landing.get(&name).copied().unwrap_or_default(),
            },
        );
    }

    let text = format!(
        "# CPU の調整済みの値（dsc tune-profile {stage} が生成。手で直さない）\n\
         # 出どころ：E2 復帰 / E3 着地合わせ / E5 行動の数値、キャラの形は data/export/{stage}\n\n{}",
        toml::to_string_pretty(&Tuned { tactics, characters })?
    );
    let path = project_root.join("rust").join("profiles").join(format!("tuned-{stage}.toml"));
    std::fs::write(&path, text)?;
    println!("wrote {}", path.display());
    Ok(true)
}
