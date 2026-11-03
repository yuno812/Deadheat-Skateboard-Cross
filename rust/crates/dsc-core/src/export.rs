//! Unityから書き出したステージとキャラのデータ（data/export/<ステージ>/）。
//! 形式は Assets/Tests/Editor/DscSim/ExportTests.cs が決める。変えたら `SCHEMA` を上げる。
//!
//! 座標：キャラの形は自分の剛体のローカル（位置と角度だけ。拡大縮小とP2の左右反転は頂点に含む）、
//! ステージの形はワールド。角度は度。

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::source::Source;

pub const SCHEMA: u32 = 2;

pub type Vec2 = [f32; 2];

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageExport {
    pub schema: u32,
    pub unity: String,
    pub stage: String,
    pub scene: String,
    pub world: World,
    /// P1, P2 の順
    pub spawn: [Vec2; 2],
    pub out_of_area: Vec2,
    pub player_scale: Vec2,
    pub shapes: Vec<Shape>,
    pub sources: Vec<Source>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct World {
    pub gravity: Vec2,
    pub dt: f32,
    pub velocity_iterations: u32,
    pub position_iterations: u32,
    pub baumgarte_scale: f32,
    #[serde(rename = "baumgarteTOIScale")]
    pub baumgarte_toi_scale: f32,
    pub default_contact_offset: f32,
    pub max_linear_correction: f32,
    pub max_angular_correction: f32,
    pub max_translation_speed: f32,
    pub max_rotation_speed: f32,
    pub bounce_threshold: f32,
    pub contact_threshold: f32,
    pub linear_sleep_tolerance: f32,
    pub angular_sleep_tolerance: f32,
    pub time_to_sleep: f32,
    pub use_sub_stepping: bool,
    pub use_sub_step_contacts: bool,
    pub use_multithreading: bool,
    pub callbacks_on_disable: bool,
    pub auto_sync_transforms: bool,
    pub queries_hit_triggers: bool,
    pub simulation_mode: String,
    /// `layer_collision_masks[i]` のビット j が立っていれば、レイヤー i と j は衝突する
    pub layer_collision_masks: Vec<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterExport {
    pub schema: u32,
    pub unity: String,
    pub stage: String,
    pub character: String,
    pub prefab: String,
    pub stats: Stats,
    /// P1（反転なし）とP2（左右反転）。凸分割の結果が変わるため、形は両方を持つ
    pub slots: Vec<Slot>,
    pub sources: Vec<Source>,
}

impl CharacterExport {
    pub fn slot(&self, slot: u8) -> Option<&Slot> {
        self.slots.iter().find(|s| s.slot == slot)
    }
}

/// `MovePlayer.Awake` が `PlayerState` から反映した後の値
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub max_speed: f32,
    pub move_force: f32,
    pub deceleration: f32,
    pub jump_force: f32,
    pub rotation_torque: f32,
    pub max_rotation_speed: f32,
    pub attack: f32,
    pub hp: f32,
    pub ult_amount: f32,
    pub ult_gain_multiplier: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Slot {
    pub slot: u8,
    pub mirrored: bool,
    pub body: Body,
    /// 有効な当たり判定が姿勢で変わるため、慣性モーメントと重心も姿勢ごとに変わる
    pub poses: BTreeMap<Pose, MassProps>,
    pub shapes: Vec<Shape>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Body {
    pub mass: f32,
    pub use_auto_mass: bool,
    pub gravity_scale: f32,
    pub linear_damping: f32,
    pub angular_damping: f32,
    pub body_type: String,
    pub collision_detection: String,
    pub sleep_mode: String,
    pub interpolation: String,
    pub constraints: String,
    pub include_layers: i32,
    pub exclude_layers: i32,
    pub pos: Vec2,
    pub angle_deg: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MassProps {
    pub inertia: f32,
    pub center_of_mass: Vec2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub enum Pose {
    Normal,
    Charge,
    Ult,
    /// どの姿勢の子にも属さず、常に有効
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ShapeKind {
    Circle,
    Capsule,
    Polygon,
    Edges,
}

/// Unity 6 の `PhysicsMaterialCombine2D`。Mean は幾何平均
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Combine {
    Average,
    Mean,
    Multiply,
    Minimum,
    Maximum,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shape {
    /// キャラ（またはステージ）の中でコライダー（Unity の Collider2D）を区別する番号。
    /// 凸分割した片はすべて同じ番号。左右の車輪のように同じ名前のオブジェクトがあるため、object では区別できない
    pub collider_index: u32,
    /// ヒエラルキー上のパス（キャラはルートより下）
    pub object: String,
    pub collider: String,
    pub pose: Pose,
    pub tag: String,
    pub layer: u32,
    /// 当たったときの処理を決めるスクリプト（WheelGroundCheck、MainCollision、AttackCollision）
    pub handlers: Vec<String>,
    pub trigger: bool,
    pub kind: ShapeKind,
    pub radius: f32,
    /// Circle は中心1点、Capsule は両端2点、Polygon は凸多角形の頂点、Edges は折れ線
    pub vertices: Vec<Vec2>,
    pub use_adjacent_start: Option<bool>,
    pub use_adjacent_end: Option<bool>,
    pub adjacent_start: Option<Vec2>,
    pub adjacent_end: Option<Vec2>,
    pub friction: f32,
    pub bounciness: f32,
    pub friction_combine: Combine,
    pub bounce_combine: Combine,
    pub density: f32,
    pub include_layers: i32,
    pub exclude_layers: i32,
}

#[derive(Debug)]
pub enum ExportError {
    Io { path: PathBuf, source: std::io::Error },
    Json { path: PathBuf, source: serde_json::Error },
    Schema { path: PathBuf, found: u32 },
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Json { path, source } => write!(f, "{}: {source}", path.display()),
            Self::Schema { path, found } => write!(
                f,
                "{}: schema {found} is not supported (expected {SCHEMA}); re-export with scripts/export-sim-data.ps1",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
            Self::Schema { .. } => None,
        }
    }
}

/// `data/export/<stage>/` の中身
pub struct StageDir {
    dir: PathBuf,
}

impl StageDir {
    pub fn new(data_dir: &Path, stage: &str) -> Self {
        Self { dir: data_dir.join("export").join(stage) }
    }

    pub fn path(&self) -> &Path {
        &self.dir
    }

    pub fn stage_path(&self) -> PathBuf {
        self.dir.join("stage.json")
    }

    pub fn character_path(&self, character: &str) -> PathBuf {
        self.dir.join(format!("{character}.json"))
    }

    pub fn load_stage(&self) -> Result<StageExport, ExportError> {
        let path = self.stage_path();
        let stage: StageExport = read_json(&path)?;
        check_schema(&path, stage.schema)?;
        Ok(stage)
    }

    pub fn load_character(&self, character: &str) -> Result<CharacterExport, ExportError> {
        let path = self.character_path(character);
        let export: CharacterExport = read_json(&path)?;
        check_schema(&path, export.schema)?;
        Ok(export)
    }

    /// 書き出されているキャラの名前（ファイル名順）
    pub fn characters(&self) -> Result<Vec<String>, ExportError> {
        let entries = std::fs::read_dir(&self.dir).map_err(|source| ExportError::Io {
            path: self.dir.clone(),
            source,
        })?;
        let mut names: Vec<String> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
            .filter_map(|p| p.file_stem().and_then(|s| s.to_str()).map(str::to_owned))
            .filter(|name| name != "stage")
            .collect();
        names.sort();
        Ok(names)
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, ExportError> {
    let text = std::fs::read_to_string(path).map_err(|source| ExportError::Io {
        path: path.to_owned(),
        source,
    })?;
    serde_json::from_str(&text).map_err(|source| ExportError::Json {
        path: path.to_owned(),
        source,
    })
}

fn check_schema(path: &Path, found: u32) -> Result<(), ExportError> {
    if found == SCHEMA {
        Ok(())
    } else {
        Err(ExportError::Schema { path: path.to_owned(), found })
    }
}
