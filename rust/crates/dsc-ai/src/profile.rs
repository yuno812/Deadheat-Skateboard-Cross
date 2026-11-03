use serde::Deserialize;

use crate::Difficulty;

/// 調整する数値。値は rust/profiles/*.toml に置き、コードに埋め込まない。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// 判断に使う状況を何ステップ遅らせるか
    pub reaction_delay_steps: u32,
    /// 操作のブレの大きさ（0でブレなし）
    pub noise: f32,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_profiles_parse() {
        for d in [Difficulty::Weak, Difficulty::Normal, Difficulty::Strong] {
            Profile::builtin(d);
        }
    }

    #[test]
    fn unknown_field_is_rejected() {
        let text = "reaction_delay_steps = 0\nnoise = 0.0\ntypo = 1\n";
        assert!(Profile::from_toml(text).is_err());
    }
}
