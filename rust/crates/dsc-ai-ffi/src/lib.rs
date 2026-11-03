//! Unityから呼ぶDLLの窓口。dsc-ai を包むだけで、判断ロジックは持たない。
//! panicはこの境界で止め、ゲームを落とさずに「入力なし」を返す。

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr::null_mut;

use dsc_ai::{Cpu, Difficulty, Input, Observation, Policy, Profile};

/// C#側はポインタとしてだけ扱う。
pub struct DscAiCpu {
    cpu: Cpu,
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
        Box::into_raw(Box::new(DscAiCpu { cpu }))
    })
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
