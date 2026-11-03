//! ランダム照合 R1 のシナリオ（仕様書「照合」）。個々の軌道ではなく、分布を Unity と比べるために使う。
//!
//! P1とP2に同じキャラを置き、それぞれに独立なランダムの初期角度と入力列を与える（1本で2サンプル）。
//! 入力は 10〜50 ステップごとに、前後移動・回転を -1/0/1 から、ジャンプを押しっぱなし（30%）かどうかを選び直す。

use dsc_core::export::StageExport;

use crate::rng::Pcg32;
use crate::{Init, InputSpan, Scenario};

pub const STEPS: u32 = 300;

/// id は R1-<キャラ>-<3桁の番号>。同じ seed なら同じシナリオになる
pub fn scenarios(stage: &StageExport, characters: &[String], count: u32, seed: u64) -> Vec<Scenario> {
    let mut out = Vec::new();
    for (ci, c) in characters.iter().enumerate() {
        for k in 0..count {
            let mut rng = Pcg32::new(seed, (ci as u64) << 32 | k as u64);
            let mut s = Scenario {
                id: format!("R1-{c}-{k:03}"),
                stage: stage.stage.clone(),
                p1: c.clone(),
                p2: c.clone(),
                steps: STEPS,
                init: vec![],
                inputs: vec![],
            };
            for slot in [1, 2] {
                s.init.push(Init {
                    slot,
                    pos: None,
                    angle_deg: Some((rng.unit() * 360.0 - 180.0).round()),
                    vel: None,
                    ang_vel_deg: None,
                });
                let mut t = 0;
                while t < STEPS {
                    let to = (t + 10 + rng.below(41)).min(STEPS);
                    let move_x = rng.below(3) as f32 - 1.0;
                    let rotate = rng.below(3) as f32 - 1.0;
                    let jump = rng.below(10) < 3;
                    if move_x != 0.0 || rotate != 0.0 || jump {
                        s.inputs.push(InputSpan {
                            slot,
                            from: t,
                            to,
                            move_x: (move_x != 0.0).then_some(move_x),
                            rotate: (rotate != 0.0).then_some(rotate),
                            jump: jump.then_some(true),
                        });
                    }
                    t = to;
                }
            }
            out.push(s);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use dsc_core::export::StageDir;

    use super::*;

    #[test]
    fn deterministic_and_valid() {
        let dir = StageDir::new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"), "R2500");
        let stage = dir.load_stage().unwrap();
        let chars = vec!["zero".to_owned(), "meme".to_owned()];
        let a = scenarios(&stage, &chars, 5, 7);
        assert_eq!(a, scenarios(&stage, &chars, 5, 7));
        assert_ne!(a, scenarios(&stage, &chars, 5, 8));
        assert_eq!(a.len(), 10);
        for s in &a {
            s.validate().unwrap();
        }
    }
}
