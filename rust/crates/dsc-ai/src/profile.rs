use serde::{Deserialize, Serialize};

use crate::Difficulty;

/// 難易度ごとのハンデ（仕様書 E6）。値は rust/profiles/*.toml に置き、コードに埋め込まない。
/// 「つよい」はハンデなし（調整済みの CPU そのまま）で、「ふつう」「よわい」の値は `dsc exp e6` が決めて書き出す。
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// 相手の状態を何ステップ遅れて見るか（反応の遅れ）
    pub reaction_delay_steps: u32,
    /// 相手の位置をずらして見る大きさ（標準偏差。狙いのブレ）
    pub aim_noise: f32,
    /// 1 ステップごとに、判断を入れ損ねて前のステップの操作のままにする確率（操作のもたつき）
    pub slip: f32,
}

impl Profile {
    pub fn from_toml(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }

    /// DLLに埋め込んだ難易度ごとの設定。
    pub fn builtin(difficulty: Difficulty) -> Self {
        let text = match difficulty {
            Difficulty::Weak => include_str!("../../../profiles/weak.toml"),
            Difficulty::Normal => include_str!("../../../profiles/normal.toml"),
            Difficulty::Strong => include_str!("../../../profiles/strong.toml"),
        };
        Self::from_toml(text).expect("builtin profile must parse")
    }

    /// ハンデが無いか（「つよい」）
    pub fn is_none(&self) -> bool {
        self.reaction_delay_steps == 0 && self.aim_noise == 0.0 && self.slip == 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_profiles_parse() {
        for d in [Difficulty::Weak, Difficulty::Normal, Difficulty::Strong] {
            Profile::builtin(d);
        }
        assert!(Profile::builtin(Difficulty::Strong).is_none());
    }

    #[test]
    fn weaker_levels_have_more_handicap() {
        let (weak, normal) = (Profile::builtin(Difficulty::Weak), Profile::builtin(Difficulty::Normal));
        assert!(weak.reaction_delay_steps >= normal.reaction_delay_steps && weak.aim_noise >= normal.aim_noise && weak.slip >= normal.slip);
        assert!(!normal.is_none());
    }

    #[test]
    fn unknown_field_is_rejected() {
        let text = "reaction_delay_steps = 0\naim_noise = 0.0\nslip = 0.0\ntypo = 1\n";
        assert!(Profile::from_toml(text).is_err());
    }
}
