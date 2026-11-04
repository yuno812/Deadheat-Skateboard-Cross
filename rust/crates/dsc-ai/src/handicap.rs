//! 難易度のハンデ（仕様書 E6）。調整済みの CPU（Brain）の前後に挟み、人がやりがちな弱さを足す。
//!
//! - 反応の遅れ：相手の状態を `reaction_delay_steps` だけ前のものとして見る。自分の状態は今のまま
//!   （自分の体の立て直しまで遅らせると、人より極端に下手になるため）
//! - 狙いのブレ：相手の位置を、正規分布でずらして見る。ずれは `AIM_HOLD_STEPS` ごとに引き直す
//!   （毎ステップ変えると打ち消し合って効かないため、しばらく同じ方向にずれ続ける）
//! - 操作のもたつき：確率 `slip` で、そのステップの判断を入れ損ね、前のステップの操作のままにする
//!
//! ハンデが無い設定（「つよい」）では、包んだ CPU とまったく同じ入力を返す。

use std::collections::VecDeque;

use crate::rng::Pcg32;
use crate::{BodyObs, Input, Observation, Policy, Profile};

/// 狙いのずれを引き直す間隔（0.6 秒）
const AIM_HOLD_STEPS: u32 = 30;
/// 縦のずれは横の半分（高さの見誤りは横より小さい）
const AIM_VERTICAL: f32 = 0.5;

pub struct Handicapped<P> {
    inner: P,
    profile: Profile,
    rng: Pcg32,
    /// 相手の状態の履歴（古い順）
    seen: VecDeque<BodyObs>,
    aim_offset: [f32; 2],
    aim_left: u32,
    last: Input,
}

impl<P: Policy> Handicapped<P> {
    pub fn new(inner: P, profile: Profile, seed: u64) -> Self {
        Self {
            inner,
            profile,
            rng: Pcg32::new(seed, 0x6469_6666), // "diff"
            seen: VecDeque::new(),
            aim_offset: [0.0; 2],
            aim_left: 0,
            last: Input::default(),
        }
    }

    pub fn inner(&self) -> &P {
        &self.inner
    }

    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    /// 試合の途中でハンデを変える（遊びながら値を決めるため）。遅れの履歴とずれはそのまま引き継ぐ
    pub fn set_profile(&mut self, profile: Profile) {
        self.profile = profile;
    }
}

impl<P: Policy> Policy for Handicapped<P> {
    fn act(&mut self, obs: &Observation) -> Input {
        if self.profile.is_none() {
            return self.inner.act(obs);
        }
        let p = self.profile;

        // 反応の遅れ
        self.seen.push_back(obs.opp);
        while self.seen.len() > p.reaction_delay_steps as usize + 1 {
            self.seen.pop_front();
        }
        let mut view = *obs;
        view.opp = *self.seen.front().expect("just pushed");

        // 狙いのブレ
        if self.aim_left == 0 {
            self.aim_offset = [self.rng.normal() * p.aim_noise, self.rng.normal() * p.aim_noise * AIM_VERTICAL];
            self.aim_left = AIM_HOLD_STEPS;
        }
        self.aim_left -= 1;
        view.opp.pos.x += self.aim_offset[0];
        view.opp.pos.y += self.aim_offset[1];

        let input = self.inner.act(&view);

        // 操作のもたつき（ウルトは押し損ねたことにする）
        let out = if self.rng.unit() < p.slip { Input { ult: false, ..self.last } } else { input };
        self.last = out;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Vec2;

    /// 見た相手の x をそのまま move_x に返す（何を見たか調べる用）
    struct Echo;

    impl Policy for Echo {
        fn act(&mut self, obs: &Observation) -> Input {
            Input { move_x: obs.opp.pos.x, rotate: obs.opp.pos.y, ..Default::default() }
        }
    }

    fn at(step: u32, x: f32) -> Observation {
        Observation { step, opp: BodyObs { pos: Vec2 { x, y: 0.0 }, ..Default::default() }, ..Default::default() }
    }

    #[test]
    fn no_handicap_passes_through() {
        let mut h = Handicapped::new(Echo, Profile::default(), 1);
        let got: Vec<f32> = (0..5).map(|i| h.act(&at(i, i as f32)).move_x).collect();
        assert_eq!(got, [0.0, 1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn sees_the_opponent_late() {
        let mut h = Handicapped::new(Echo, Profile { reaction_delay_steps: 2, ..Default::default() }, 1);
        let got: Vec<f32> = (0..6).map(|i| h.act(&at(i, i as f32)).move_x).collect();
        // はじめは最初の状態のまま、そのあとは 2 ステップ前
        assert_eq!(got, [0.0, 0.0, 0.0, 1.0, 2.0, 3.0]);
    }

    #[test]
    fn aim_is_off_by_a_held_offset() {
        let mut h = Handicapped::new(Echo, Profile { aim_noise: 1.0, ..Default::default() }, 1);
        let xs: Vec<f32> = (0..AIM_HOLD_STEPS * 3).map(|i| h.act(&at(i, 0.0)).move_x).collect();
        // 30 ステップは同じずれ、その次で引き直す
        assert!(xs[..AIM_HOLD_STEPS as usize].iter().all(|&x| x == xs[0]) && xs[0] != 0.0);
        assert_ne!(xs[0], xs[AIM_HOLD_STEPS as usize]);
    }

    #[test]
    fn slips_repeat_the_last_input() {
        let mut h = Handicapped::new(Echo, Profile { slip: 0.5, ..Default::default() }, 7);
        let got: Vec<f32> = (0..200).map(|i| h.act(&at(i, i as f32)).move_x).collect();
        let repeats = got.windows(2).filter(|w| w[0] == w[1]).count();
        // 約半分が前のステップのまま
        assert!((70..130).contains(&repeats), "{repeats}");
    }
}
