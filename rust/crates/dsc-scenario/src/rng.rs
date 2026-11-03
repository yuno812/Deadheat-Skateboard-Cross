//! PCG32（pcg32_random_r）。同じ種で同じ列になることを保証するため自前で持つ（仕様書：乱数は試合ごとに種を渡す）。

#[derive(Debug, Clone)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    /// pcg32_srandom_r(seed, stream) と同じ
    pub fn new(seed: u64, stream: u64) -> Self {
        let mut rng = Self { state: 0, inc: (stream << 1) | 1 };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        xorshifted.rotate_right((old >> 59) as u32)
    }

    /// 0 以上 n 未満の一様な整数（偏りが出ないよう、端の値は引き直す）
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0);
        let threshold = n.wrapping_neg() % n;
        loop {
            let r = self.next_u32();
            if r >= threshold {
                return r % n;
            }
        }
    }

    /// 0 以上 1 未満の一様な実数
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_pcg32_reference_output() {
        // pcg32-demo（pcg32_srandom_r(&rng, 42, 54)）の最初の6つ
        let mut rng = Pcg32::new(42, 54);
        let got: Vec<u32> = (0..6).map(|_| rng.next_u32()).collect();
        assert_eq!(got, [0xa15c02b7, 0x7b47f409, 0xba1d3330, 0x83d2f293, 0xbfa4784b, 0xcbed606e]);
    }

    #[test]
    fn below_stays_in_range() {
        let mut rng = Pcg32::new(1, 1);
        assert!((0..1000).all(|_| rng.below(3) < 3));
        assert!((0..1000).map(|_| rng.unit()).all(|u| (0.0..1.0).contains(&u)));
    }
}
