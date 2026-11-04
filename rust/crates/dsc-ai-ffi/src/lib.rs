//! Unityから呼ぶDLLの窓口。dsc-ai を包むだけで、判断ロジックは持たない。
//! panicはこの境界で止め、ゲームを落とさずに「入力なし」を返す。

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr::null_mut;

use dsc_ai::brain::Brain;
use dsc_ai::terrain::Terrain;
use dsc_ai::tuned::{Stats, brain_for};
use dsc_ai::{BrainSetup, Cpu, Difficulty, Input, Observation, Policy, Profile};

/// C#側はポインタとしてだけ扱う。
pub struct DscAiCpu {
    cpu: Box<dyn Policy>,
}

/// C#側の構造体の大きさと比べ、古いDLLを読み込んでいないか確かめるために使う。
#[unsafe(no_mangle)]
pub extern "C" fn dsc_ai_observation_size() -> u32 {
    size_of::<Observation>() as u32
}

#[unsafe(no_mangle)]
pub extern "C" fn dsc_ai_input_size() -> u32 {
    size_of::<Input>() as u32
}

/// 難易度が不正なら null を返す。
#[unsafe(no_mangle)]
pub extern "C" fn dsc_ai_create(difficulty: u8, seed: u64) -> *mut DscAiCpu {
    let Ok(difficulty) = Difficulty::try_from(difficulty) else {
        return null_mut();
    };
    catch_unwind(|| {
        let cpu = Cpu::new(Profile::builtin(difficulty), seed);
        Box::into_raw(Box::new(DscAiCpu { cpu: Box::new(cpu) }))
    })
    .unwrap_or(null_mut())
}

/// 対戦用の CPU を作る。character はキャラ名（Prefab のファイル名の小文字、UTF-8）、調整済みの値は DLL に埋め込み。
/// polygon_xy はステージの当たり判定の凸多角形の頂点（ワールド座標、x0, y0, x1, y1, …）を、
/// polygon_counts は多角形ごとの頂点数を polygon_count 個並べたもの。失敗したら null を返す。
/// difficulty と seed は今は使わない（難易度は E6 で入れる）。
///
/// # Safety
/// 各ポインタは、書かれた長さのぶん読めること（長さが 0 なら null でもよい）。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dsc_ai_create_brain(
    character: *const u8,
    character_len: usize,
    difficulty: u8,
    seed: u64,
    setup: *const BrainSetup,
    polygon_xy: *const f32,
    polygon_counts: *const u32,
    polygon_count: u32,
) -> *mut DscAiCpu {
    if setup.is_null() || Difficulty::try_from(difficulty).is_err() {
        return null_mut();
    }
    let _ = seed;
    catch_unwind(AssertUnwindSafe(|| {
        let name = if character.is_null() || character_len == 0 {
            String::new()
        } else {
            String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(character, character_len) }).into_owned()
        };
        let counts: &[u32] = if polygon_counts.is_null() { &[] } else { unsafe { std::slice::from_raw_parts(polygon_counts, polygon_count as usize) } };
        let total: usize = counts.iter().map(|&c| c as usize).sum();
        let xy: &[f32] = if polygon_xy.is_null() { &[] } else { unsafe { std::slice::from_raw_parts(polygon_xy, total * 2) } };
        let mut polygons = Vec::with_capacity(counts.len());
        let mut at = 0;
        for &c in counts {
            let poly: Vec<[f32; 2]> = (0..c as usize).map(|i| [xy[(at + i) * 2], xy[(at + i) * 2 + 1]]).collect();
            at += c as usize;
            polygons.push(poly);
        }
        let terrain = Terrain::from_polygons(polygons.iter().map(|p| p.as_slice()));
        let s = unsafe { *setup };
        let stats = Stats { jump_force: s.jump_force, max_speed: s.max_speed, deceleration: s.deceleration, gravity_y: s.gravity_y, dt: s.dt };
        let brain: Brain = brain_for(&name.to_lowercase(), stats, terrain);
        Box::into_raw(Box::new(DscAiCpu { cpu: Box::new(brain) }))
    }))
    .unwrap_or(null_mut())
}

/// 成功なら true。null や panic のときは `out` に入力なしを書いて false を返す。
///
/// # Safety
/// `cpu` は `dsc_ai_create` が返した破棄前のポインタか null。`obs` と `out` は有効なポインタか null。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dsc_ai_act(
    cpu: *mut DscAiCpu,
    obs: *const Observation,
    out: *mut Input,
) -> bool {
    if out.is_null() {
        return false;
    }
    let input = if cpu.is_null() || obs.is_null() {
        None
    } else {
        catch_unwind(AssertUnwindSafe(|| unsafe { (*cpu).cpu.act(&*obs) })).ok()
    };
    unsafe { out.write(input.unwrap_or_default()) };
    input.is_some()
}

/// # Safety
/// `cpu` は `dsc_ai_create` が返した破棄前のポインタか null。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dsc_ai_destroy(cpu: *mut DscAiCpu) {
    if !cpu.is_null() {
        drop(unsafe { Box::from_raw(cpu) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brain_plays_from_the_dll() {
        let name = b"zero";
        let setup = BrainSetup { jump_force: 9.0, max_speed: 5.0, deceleration: 1.0, gravity_y: -9.81, dt: 0.02 };
        // 床：x = -10..10、y = -5..-4 の長方形1つ
        let xy = [-10.0f32, -5.0, 10.0, -5.0, 10.0, -4.0, -10.0, -4.0];
        let counts = [4u32];
        let cpu = unsafe { dsc_ai_create_brain(name.as_ptr(), name.len(), 1, 0, &setup, xy.as_ptr(), counts.as_ptr(), 1) };
        assert!(!cpu.is_null());
        let mut obs = Observation::default();
        obs.me.pos.y = -2.8;
        obs.me.ground_count = 2;
        obs.me.facing = 1.0;
        obs.opp.pos.x = 5.0;
        obs.opp.facing = -1.0;
        obs.gravity_up.y = 1.0;
        let mut out = Input::default();
        assert!(unsafe { dsc_ai_act(cpu, &obs, &mut out) });
        // 遠くの相手へ：溜めながら前進して右へ傾ける
        assert!(out.jump && out.move_x == 1.0, "{out:?}");
        // zero は、正面の相手（落ちてきて弾の線に入る）には撃ち、背中側の相手には撃たない
        obs.me.hp = 3.0;
        obs.me.ult = 170.0;
        obs.me.ult_max = 170.0;
        assert!(unsafe { dsc_ai_act(cpu, &obs, &mut out) });
        assert!(out.ult, "{out:?}");
        obs.opp.pos.x = -5.0;
        assert!(unsafe { dsc_ai_act(cpu, &obs, &mut out) });
        assert!(!out.ult, "{out:?}");
        obs.opp.pos.x = 5.0;
        unsafe { dsc_ai_destroy(cpu) };

        // cubu はゲージが満タンならすぐ撃つ
        let name = b"cubu";
        let cpu = unsafe { dsc_ai_create_brain(name.as_ptr(), name.len(), 1, 0, &setup, xy.as_ptr(), counts.as_ptr(), 1) };
        assert!(!cpu.is_null());
        obs.me.ult = 100.0;
        obs.me.ult_max = 100.0;
        assert!(unsafe { dsc_ai_act(cpu, &obs, &mut out) });
        assert!(out.ult, "{out:?}");
        unsafe { dsc_ai_destroy(cpu) };
    }

    #[test]
    fn create_rejects_unknown_difficulty() {
        assert!(dsc_ai_create(3, 0).is_null());
    }

    #[test]
    fn act_round_trip() {
        for difficulty in 0..=2 {
            let cpu = dsc_ai_create(difficulty, 42);
            assert!(!cpu.is_null());
            let obs = Observation::default();
            let mut out = Input { move_x: 9.0, ..Input::default() };
            assert!(unsafe { dsc_ai_act(cpu, &obs, &mut out) });
            assert_eq!(out, Input::default());
            unsafe { dsc_ai_destroy(cpu) };
        }
    }

    #[test]
    fn act_with_null_writes_no_input() {
        let mut out = Input { jump: true, ..Input::default() };
        assert!(!unsafe { dsc_ai_act(null_mut(), &Observation::default(), &mut out) });
        assert_eq!(out, Input::default());
        assert!(!unsafe { dsc_ai_act(null_mut(), std::ptr::null(), null_mut()) });
        unsafe { dsc_ai_destroy(null_mut()) };
    }
}
