//! 照合用シナリオ S1〜S11（仕様書「照合」の表）。
//!
//! R2500 は左右2つのお椀を中央の丘で仕切った閉じた地形で、仕様書が想定した平地はない。
//! そこで、P1とP2に同じキャラを置き、左右対称の入力を同時に与えて、反転なし・反転ありを1本で確かめる。
//! 2体はそれぞれ自分のお椀の中で動き、S8とS9以外では出会わない。

use dsc_core::export::StageExport;

use crate::{Init, InputSpan, Scenario};

/// スポーン地点から落ちて着地し、止まるまで待つステップ数
const SETTLE: u32 = 50;
const LEFT: f32 = 1.0;
const RIGHT: f32 = -1.0;

pub fn scenarios(stage: &StageExport, characters: &[String]) -> Vec<Scenario> {
    characters.iter().flat_map(|c| for_character(stage, c)).collect()
}

fn for_character(stage: &StageExport, c: &str) -> Vec<Scenario> {
    let mut out = Vec::new();
    let new = |id: String, steps: u32| Scenario {
        id,
        stage: stage.stage.clone(),
        p1: c.to_owned(),
        p2: c.to_owned(),
        steps,
        init: vec![],
        inputs: vec![],
    };

    // S1：入力なし。落下、着地、静止
    out.push(new(format!("S1-{c}"), 150));

    // S2：押し続けて加速。外側（後ろ）へ進ませ、中央で2体が出会わないようにする
    let mut s = new(format!("S2-{c}"), SETTLE + 150);
    s.inputs = both(SETTLE, SETTLE + 150, |span| span.move_x = Some(-1.0));
    out.push(s);

    // S3：ジャンプを10ステップ押して離す
    let mut s = new(format!("S3-{c}"), SETTLE + 150);
    s.inputs = both(SETTLE, SETTLE + 10, |span| span.jump = Some(true));
    out.push(s);

    // S4：押している最後の k ステップだけ回して板を傾け、傾いたまま離す
    for (name, dir) in [("L", LEFT), ("R", RIGHT)] {
        for k in [5, 10] {
            let mut s = new(format!("S4-{c}-rot{name}{k}"), SETTLE + 150);
            s.inputs = both(SETTLE, SETTLE + 10, |span| span.jump = Some(true));
            s.inputs.extend(both(SETTLE + 10 - k, SETTLE + 10, |span| span.rotate = Some(dir)));
            out.push(s);
        }
    }

    // S5：空中高くで1秒回し続ける（トルクの単位、慣性モーメント、900度/秒の制限）
    for (name, dir) in [("L", LEFT), ("R", RIGHT)] {
        let mut s = new(format!("S5-{c}-rot{name}"), 100);
        s.init = [1, 2]
            .map(|slot| {
                let [x, y] = stage.spawn[slot as usize - 1];
                Init { pos: Some([x, y + 4.0]), ..init(slot) }
            })
            .to_vec();
        s.inputs = both(0, 50, |span| span.rotate = Some(dir));
        out.push(s);
    }

    // S6：逆さまで着地してから、5秒回し続ける
    for (name, dir) in [("L", LEFT), ("R", RIGHT)] {
        let mut s = new(format!("S6-{c}-rot{name}"), SETTLE + 250);
        s.init = [1, 2].map(|slot| Init { angle_deg: Some(180.0), ..init(slot) }).to_vec();
        s.inputs = both(SETTLE, SETTLE + 250, |span| span.rotate = Some(dir));
        out.push(s);
    }

    // S7：横倒しで着地してから、5秒回し続ける
    for angle in [90.0, -90.0] {
        for (name, dir) in [("L", LEFT), ("R", RIGHT)] {
            let mut s = new(format!("S7-{c}-a{angle:+}-rot{name}"), SETTLE + 250);
            s.init = [1, 2].map(|slot| Init { angle_deg: Some(angle), ..init(slot) }).to_vec();
            s.inputs = both(SETTLE, SETTLE + 250, |span| span.rotate = Some(dir));
            out.push(s);
        }
    }

    // S8：向かい合って前進し、中央の丘でぶつかる
    let mut s = new(format!("S8-{c}"), SETTLE + 200);
    s.inputs = both(SETTLE, SETTLE + 200, |span| span.move_x = Some(1.0));
    out.push(s);

    // S9：P1をP2の真上から落とす（踏みつけ）
    let mut s = new(format!("S9-{c}"), 150);
    let [x2, y2] = stage.spawn[1];
    s.init = vec![Init { pos: Some([x2, y2 + 3.0]), ..init(1) }];
    out.push(s);

    // S10：場外に置く。R2500は壁で閉じていて走って出られないため、最初から境界の外に置いて判定を確かめる
    let mut s = new(format!("S10-{c}"), 100);
    let [ox, _] = stage.out_of_area;
    s.init = vec![
        Init { pos: Some([ox.abs() + 0.5, 0.0]), ..init(1) },
        Init { pos: Some([-(ox.abs() + 0.5), 0.0]), ..init(2) },
    ];
    out.push(s);

    // S11：ジャンプを押しっぱなしにして、姿勢の切り替えで接地カウントが変わるかを見る
    let mut s = new(format!("S11-{c}"), SETTLE + 200);
    s.inputs = both(SETTLE, SETTLE + 100, |span| span.jump = Some(true));
    out.push(s);

    out
}

fn init(slot: u8) -> Init {
    Init { slot, pos: None, angle_deg: None, vel: None, ang_vel_deg: None }
}

/// P1とP2に同じ入力区間を作る（入力の値は各自の向きで解釈されるので、動きは左右対称になる）
fn both(from: u32, to: u32, set: impl Fn(&mut InputSpan)) -> Vec<InputSpan> {
    [1, 2]
        .map(|slot| {
            let mut span = InputSpan { slot, from, to, move_x: None, rotate: None, jump: None };
            set(&mut span);
            span
        })
        .to_vec()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::Path;

    use dsc_core::export::StageDir;

    use super::*;

    #[test]
    fn all_scenarios_are_valid_and_unique() {
        let dir = StageDir::new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"), "R2500");
        let stage = dir.load_stage().unwrap();
        let all = scenarios(&stage, &dir.characters().unwrap());
        assert_eq!(all.len(), 7 * 19);
        let mut ids = HashSet::new();
        for s in &all {
            s.validate().unwrap();
            assert!(ids.insert(s.id.clone()), "duplicate id {}", s.id);
        }
    }
}
