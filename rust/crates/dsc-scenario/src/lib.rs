//! シナリオの形式、照合用シナリオとランダムシナリオの生成、トレースCSVの読み書き。
//! シナリオはUnity（Assets/Tests/Editor/DscSim/Scenario.cs）とRustの両方で同じファイルを再生する。

pub mod random;
pub mod rng;
mod scenario;
pub mod trace;
pub mod verify;

pub use scenario::{Init, InputSpan, Scenario, ScenarioError, StepInput};
