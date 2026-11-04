//! CPUの判断（状況を受け取って入力を返す）と、角度制御・ジャンプ予測などの部品。
//! シミュレーター（dsc-cli）からはそのまま、ゲームからは dsc-ai-ffi のDLL経由で使う。

mod abi;
mod cpu;
mod profile;
pub mod recovery;

pub use abi::{BodyObs, Difficulty, Input, Observation, Pose, Vec2};
pub use cpu::Cpu;
pub use profile::Profile;

pub trait Policy {
    fn act(&mut self, obs: &Observation) -> Input;
}
