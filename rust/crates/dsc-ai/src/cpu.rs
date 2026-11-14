use crate::{Input, Observation, Policy, Profile};

/// ゲームとシミュレーターの両方で使うCPU本体。
/// 今は何もしない（入力なしを返す）。判断ロジックはCPU調整の段階で入れる。
pub struct Cpu {
    profile: Profile,
    seed: u64,
}

impl Cpu {
    pub fn new(profile: Profile, seed: u64) -> Self {
        Self { profile, seed }
    }

    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }
}

impl Policy for Cpu {
    fn act(&mut self, _obs: &Observation) -> Input {
        Input::default()
    }
}
