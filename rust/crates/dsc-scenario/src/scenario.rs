use std::fmt;

use serde::{Deserialize, Serialize};

pub type Vec2 = [f32; 2];

/// 1本のシナリオ。ステップは物理ステップ（dt = 0.02秒）の番号で、0 から `steps - 1` まで。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Scenario {
    pub id: String,
    pub stage: String,
    pub p1: String,
    pub p2: String,
    pub steps: u32,
    /// 書かれていないプレイヤーはスポーン地点から、静止した状態で始める
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub init: Vec<Init>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<InputSpan>,
}

/// 最初のステップの前に反映する状態。省略した項目は、位置と角度はそのまま、速度は0。
/// 位置は Transform から、角度と速度は剛体へ直接入れる（Transform の角度は最初のステップの後に書き戻される）。
/// ウルトゲージ用の「前回の位置」もこの位置にそろえる（瞬間移動でゲージが溜まらないように）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Init {
    pub slot: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<Vec2>,
    /// 剛体の角度。ワールドで反時計回りが正（トレースの rot、普段は angle と同じ）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub angle_deg: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vel: Option<Vec2>,
    /// ワールドで反時計回りが正（トレースの w と同じ）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ang_vel_deg: Option<f32>,
}

/// `from` 以上 `to` 未満のステップの入力。項目ごとに、そのステップを含む最後の区間で
/// 指定された値を使う（どの区間も指定していなければ 0 / false）。
/// 値はゲームの入力と同じ意味：`move_x` は自分の前方が正、`rotate` は左回転（キーボードの左）が正。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputSpan {
    pub slot: u8,
    pub from: u32,
    pub to: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub move_x: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotate: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jump: Option<bool>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StepInput {
    pub move_x: f32,
    pub rotate: f32,
    pub jump: bool,
}

impl Scenario {
    pub fn input(&self, slot: u8, step: u32) -> StepInput {
        let mut input = StepInput::default();
        for span in self
            .inputs
            .iter()
            .filter(|s| s.slot == slot && s.from <= step && step < s.to)
        {
            if let Some(v) = span.move_x {
                input.move_x = v;
            }
            if let Some(v) = span.rotate {
                input.rotate = v;
            }
            if let Some(v) = span.jump {
                input.jump = v;
            }
        }
        input
    }

    pub fn init(&self, slot: u8) -> Option<&Init> {
        self.init.iter().find(|i| i.slot == slot)
    }

    pub fn validate(&self) -> Result<(), ScenarioError> {
        let err = |msg: String| Err(ScenarioError { id: self.id.clone(), msg });
        if self.steps == 0 {
            return err("steps must be > 0".into());
        }
        for (i, init) in self.init.iter().enumerate() {
            if !matches!(init.slot, 1 | 2) {
                return err(format!("init[{i}]: slot must be 1 or 2"));
            }
            if self.init[..i].iter().any(|o| o.slot == init.slot) {
                return err(format!("init[{i}]: slot {} appears twice", init.slot));
            }
        }
        for (i, span) in self.inputs.iter().enumerate() {
            if !matches!(span.slot, 1 | 2) {
                return err(format!("inputs[{i}]: slot must be 1 or 2"));
            }
            if span.from >= span.to || span.to > self.steps {
                return err(format!("inputs[{i}]: need from < to <= steps"));
            }
        }
        Ok(())
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("scenario serializes") + "\n"
    }

    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioError {
    pub id: String,
    pub msg: String,
}

impl fmt::Display for ScenarioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.id, self.msg)
    }
}

impl std::error::Error for ScenarioError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(from: u32, to: u32) -> InputSpan {
        InputSpan { slot: 1, from, to, move_x: None, rotate: None, jump: None }
    }

    fn scenario(inputs: Vec<InputSpan>) -> Scenario {
        Scenario {
            id: "t".into(),
            stage: "R2500".into(),
            p1: "zero".into(),
            p2: "zero".into(),
            steps: 100,
            init: vec![],
            inputs,
        }
    }

    #[test]
    fn later_span_wins_per_field() {
        let s = scenario(vec![
            InputSpan { jump: Some(true), ..span(10, 20) },
            InputSpan { rotate: Some(1.0), ..span(15, 20) },
            InputSpan { jump: Some(false), ..span(18, 19) },
        ]);
        assert_eq!(s.input(1, 9), StepInput::default());
        assert_eq!(s.input(1, 10), StepInput { jump: true, ..Default::default() });
        assert_eq!(s.input(1, 15), StepInput { rotate: 1.0, jump: true, ..Default::default() });
        assert_eq!(s.input(1, 18), StepInput { rotate: 1.0, jump: false, ..Default::default() });
        assert_eq!(s.input(1, 20), StepInput::default());
        assert_eq!(s.input(2, 15), StepInput::default());
    }

    #[test]
    fn json_round_trip_omits_unset_fields() {
        let s = scenario(vec![InputSpan { move_x: Some(1.0), ..span(0, 10) }]);
        let json = s.to_json();
        assert!(!json.contains("rotate"));
        assert!(!json.contains("init"));
        assert_eq!(Scenario::from_json(&json).unwrap(), s);
    }

    #[test]
    fn validate_rejects_bad_spans() {
        assert!(scenario(vec![span(5, 5)]).validate().is_err());
        assert!(scenario(vec![span(0, 101)]).validate().is_err());
        assert!(scenario(vec![InputSpan { slot: 3, ..span(0, 1) }]).validate().is_err());
        assert!(scenario(vec![span(0, 100)]).validate().is_ok());
    }
}
