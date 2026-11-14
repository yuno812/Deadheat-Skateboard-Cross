//! C#と共有する境界の型。dsc-ai-ffi の build.rs が、このファイルからC#の構造体を生成する。
//! フィールドを変えたら、DLLを作り直してUnityに入れ直すこと（scripts/build-ai.ps1）。

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Pose {
    #[default]
    Normal = 0,
    Charge = 1,
    Ult = 2,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BodyObs {
    pub pos: Vec2,
    pub vel: Vec2,
    /// ワールドの上向き（+y）から反時計回り、-180..180。位置・速度もワールドのまま渡す
    pub angle_deg: f32,
    pub ang_vel_deg: f32,
    pub ground_count: u8,
    pub pose: Pose,
    pub hp: f32,
    pub ult: f32,
    pub ult_max: f32,
    /// P1 = +1, P2 = -1
    pub facing: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Observation {
    pub step: u32,
    /// 自分
    pub me: BodyObs,
    /// 相手
    pub opp: BodyObs,
    /// 自分にかかる重力の逆向き（重力反転ステージ用）。下向き（y < 0）なら、CPU は世界を180°回して判断する
    pub gravity_up: Vec2,
    /// 場外の境界
    pub bounds: Vec2,
}

/// 対戦用 CPU を作るときにゲームから渡す、キャラの今の能力値と物理の設定
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BrainSetup {
    pub jump_force: f32,
    pub max_speed: f32,
    /// MovePlayer.deceleration（moveForce / 10）
    pub deceleration: f32,
    pub gravity_y: f32,
    /// Time.fixedDeltaTime
    pub dt: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Input {
    pub move_x: f32,
    pub rotate: f32,
    pub jump: bool,
    pub ult: bool,
}

/// DLLでは u8 で受け取り、`TryFrom` で検証してから使う。
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    Weak = 0,
    Normal = 1,
    Strong = 2,
}

impl TryFrom<u8> for Difficulty {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Weak),
            1 => Ok(Self::Normal),
            2 => Ok(Self::Strong),
            other => Err(other),
        }
    }
}
