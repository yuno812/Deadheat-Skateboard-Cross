//! Unityから呼ぶDLLの窓口。dsc-ai を包むだけで、判断ロジックは持たない。
//! panicはこの境界で止め、ゲームを落とさずに「入力なし」を返す。

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr::null_mut;

use dsc_ai::brain::Brain;
use dsc_ai::terrain::Terrain;
use dsc_ai::tuned::{Stats, brain_for};
use dsc_ai::handicap::Handicapped;
use dsc_ai::{BrainSetup, Cpu, Difficulty, Input, Observation, Policy, Profile};

/// C#側はポインタとしてだけ扱う。
pub struct DscAiCpu {
    inner: Inner,
}

enum Inner {
    /// dsc_ai_create の、何もしない CPU（DLL の読み込み確認用）
    Idle(Cpu),
    /// dsc_ai_create_brain の、対戦用の CPU（難易度のハンデ付き）
    Brain(Box<Handicapped<Brain>>),
}

impl Inner {
    fn act(&mut self, obs: &Observation) -> Input {
        match self {
            Inner::Idle(cpu) => cpu.act(obs),
            Inner::Brain(cpu) => cpu.act(obs),
        }
    }
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
        Box::into_raw(Box::new(DscAiCpu { inner: Inner::Idle(cpu) }))
    })
    .unwrap_or(null_mut())
}

/// 対戦用の CPU を作る。character はキャラ名（Prefab のファイル名の小文字、UTF-8）、調整済みの値は DLL に埋め込み。
/// polygon_xy はステージの当たり判定の凸多角形の頂点（ワールド座標、x0, y0, x1, y1, …）を、
/// polygon_counts は多角形ごとの頂点数を polygon_count 個並べたもの。失敗したら null を返す。
/// difficulty は難易度（0 よわい・1 ふつう・2 つよい）。「つよい」は調整済みの CPU そのまま、ほかはハンデを足す
/// （rust/profiles/*.toml、dsc exp e6 で決めた値）。seed はハンデの乱数の種。
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
    let Some(setup_ptr) = (!setup.is_null()).then_some(setup) else { return null_mut() };
    let Ok(difficulty) = Difficulty::try_from(difficulty) else { return null_mut() };
    let setup = setup_ptr;
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
        let cpu = Handicapped::new(brain, Profile::builtin(difficulty), seed);
        Box::into_raw(Box::new(DscAiCpu { inner: Inner::Brain(Box::new(cpu)) }))
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
        catch_unwind(AssertUnwindSafe(|| unsafe { (*cpu).inner.act(&*obs) })).ok()
    };
    unsafe { out.write(input.unwrap_or_default()) };
    input.is_some()
}

/// 対戦用の CPU の、今のハンデ（反応の遅れのステップ数、狙いのブレ、もたつきの確率）を読む。
/// 対戦用でない・null なら false。
///
/// # Safety
/// `cpu` は `dsc_ai_create_brain` が返した破棄前のポインタか null。出力のポインタは有効なものか null。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dsc_ai_get_handicap(
    cpu: *const DscAiCpu,
    reaction_delay_steps: *mut u32,
    aim_noise: *mut f32,
    slip: *mut f32,
) -> bool {
    let Some(DscAiCpu { inner: Inner::Brain(brain) }) = (unsafe { cpu.as_ref() }) else { return false };
    if reaction_delay_steps.is_null() || aim_noise.is_null() || slip.is_null() {
        return false;
    }
    let p = brain.profile();
    unsafe {
        reaction_delay_steps.write(p.reaction_delay_steps);
        aim_noise.write(p.aim_noise);
        slip.write(p.slip);
    }
    true
}

/// 対戦用の CPU のハンデを、試合の途中で変える（遊びながら難易度の値を決めるため）。
/// 値は 0 以上（もたつきは 1 以下）に丸める。対戦用でない・null なら false。
///
/// # Safety
/// `cpu` は `dsc_ai_create_brain` が返した破棄前のポインタか null。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dsc_ai_set_handicap(cpu: *mut DscAiCpu, reaction_delay_steps: u32, aim_noise: f32, slip: f32) -> bool {
    let Some(DscAiCpu { inner: Inner::Brain(brain) }) = (unsafe { cpu.as_mut() }) else { return false };
    let clean = |v: f32| if v.is_finite() { v.max(0.0) } else { 0.0 };
    brain.set_profile(Profile { reaction_delay_steps, aim_noise: clean(aim_noise), slip: clean(slip).min(1.0) });
    true
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
        let cpu = unsafe { dsc_ai_create_brain(name.as_ptr(), name.len(), 2, 0, &setup, xy.as_ptr(), counts.as_ptr(), 1) };
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
        let cpu = unsafe { dsc_ai_create_brain(name.as_ptr(), name.len(), 2, 0, &setup, xy.as_ptr(), counts.as_ptr(), 1) };
        assert!(!cpu.is_null());
        obs.me.ult = 100.0;
        obs.me.ult_max = 100.0;
        assert!(unsafe { dsc_ai_act(cpu, &obs, &mut out) });
        assert!(out.ult, "{out:?}");
        unsafe { dsc_ai_destroy(cpu) };

        // よわい・ふつうも作れて、ハンデ付きで動く。知らない難易度は作れない
        for difficulty in 0..=1 {
            let cpu = unsafe { dsc_ai_create_brain(name.as_ptr(), name.len(), difficulty, 9, &setup, xy.as_ptr(), counts.as_ptr(), 1) };
            assert!(!cpu.is_null());
            assert!((0..50).all(|_| unsafe { dsc_ai_act(cpu, &obs, &mut out) }));
            unsafe { dsc_ai_destroy(cpu) };
        }
        assert!(unsafe { dsc_ai_create_brain(name.as_ptr(), name.len(), 3, 0, &setup, xy.as_ptr(), counts.as_ptr(), 1) }.is_null());
    }

    #[test]
    fn handicap_can_be_read_and_changed() {
        let name = b"zero";
        let setup = BrainSetup { jump_force: 9.0, max_speed: 5.0, deceleration: 1.0, gravity_y: -9.81, dt: 0.02 };
        let xy = [-10.0f32, -5.0, 10.0, -5.0, 10.0, -4.0, -10.0, -4.0];
        let counts = [4u32];
        let cpu = unsafe { dsc_ai_create_brain(name.as_ptr(), name.len(), 1, 0, &setup, xy.as_ptr(), counts.as_ptr(), 1) };
        let (mut delay, mut aim, mut slip) = (0u32, 0f32, 0f32);
        assert!(unsafe { dsc_ai_get_handicap(cpu, &mut delay, &mut aim, &mut slip) });
        let normal = Profile::builtin(Difficulty::Normal);
        assert_eq!((delay, aim, slip), (normal.reaction_delay_steps, normal.aim_noise, normal.slip));
        // 変えたら読み直せる。おかしな値は丸める
        assert!(unsafe { dsc_ai_set_handicap(cpu, 3, -1.0, 5.0) });
        assert!(unsafe { dsc_ai_get_handicap(cpu, &mut delay, &mut aim, &mut slip) });
        assert_eq!((delay, aim, slip), (3, 0.0, 1.0));
        unsafe { dsc_ai_destroy(cpu) };
        // 何もしない CPU と null には効かない
        let idle = dsc_ai_create(1, 0);
        assert!(!unsafe { dsc_ai_set_handicap(idle, 1, 0.0, 0.0) });
        assert!(!unsafe { dsc_ai_get_handicap(null_mut(), &mut delay, &mut aim, &mut slip) });
        unsafe { dsc_ai_destroy(idle) };
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
