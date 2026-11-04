//! 調整済みの値（rust/profiles/tuned-R2500.toml、dsc tune-profile が生成）。DLL に埋め込み、ゲームでもそのまま使う。
//! 中身は E2（復帰）・E3（着地合わせ）・E5（行動）の結果と、書き出しから求めたキャラの形の値。

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::brain::{Brain, Knowledge, Tactics};
use crate::jump::JumpModel;
use crate::landing::{LandingParams, Wheels};
use crate::recovery::RecoveryParams;
use crate::terrain::Terrain;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tuned {
    pub tactics: Tactics,
    pub characters: BTreeMap<String, CharacterTuning>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterTuning {
    /// 体の中心から車輪の下端までの距離（通常の姿勢）
    pub ride_height: f32,
    /// 剛体のローカル座標での重心（通常の姿勢）
    pub center_of_mass: [f32; 2],
    pub recovery: RecoveryParams,
    pub landing: LandingParams,
}

impl Default for CharacterTuning {
    /// 調整していないキャラの仮の値
    fn default() -> Self {
        Self { ride_height: 1.2, center_of_mass: [0.0, -0.5], recovery: RecoveryParams::default(), landing: LandingParams::default() }
    }
}

/// ゲームから毎試合わたす、キャラの今の能力値と物理の設定
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stats {
    pub jump_force: f32,
    pub max_speed: f32,
    pub deceleration: f32,
    pub gravity_y: f32,
    pub dt: f32,
}

pub fn tuned() -> &'static Tuned {
    static TUNED: OnceLock<Tuned> = OnceLock::new();
    TUNED.get_or_init(|| toml::from_str(include_str!("../../../profiles/tuned-R2500.toml")).expect("tuned-R2500.toml must parse"))
}

/// キャラ名（Prefab のファイル名の小文字）から、調整済みの値で CPU を作る。知らないキャラは仮の値
pub fn brain_for(character: &str, stats: Stats, terrain: Terrain) -> Brain {
    let t = tuned();
    let c = t.characters.get(character).copied().unwrap_or_default();
    let knowledge = Knowledge {
        recovery: c.recovery,
        landing: c.landing,
        wheels: Wheels { ride_height: c.ride_height },
        jump: JumpModel {
            jump_force: stats.jump_force,
            max_speed: stats.max_speed,
            deceleration: stats.deceleration,
            center_of_mass: c.center_of_mass,
            gravity_y: stats.gravity_y,
            dt: stats.dt,
        },
    };
    Brain::new(knowledge, t.tactics, terrain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_profile_parses_and_has_all_characters() {
        let t = tuned();
        for c in ["cubu", "hitsuji", "maiden", "meme", "nagisa", "okuda", "zero"] {
            assert!(t.characters.contains_key(c), "{c}");
        }
    }
}
