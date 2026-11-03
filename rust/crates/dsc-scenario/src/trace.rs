//! トレースCSV（1行＝1ステップ×1プレイヤー）。Unity側は Assets/Tests/Editor/DscSim/TraceRecorder.cs が書く。
//! 各行は、そのステップの物理計算と衝突の処理が終わった後の状態。

use std::fmt;
use std::str::FromStr;

pub const HEADER: &str = "step,slot,x,y,angle,rot,vx,vy,w,ground,pose,hp,ult";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pose {
    Normal,
    Charge,
    Ult,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceRow {
    pub step: u32,
    pub slot: u8,
    pub x: f32,
    pub y: f32,
    /// 見た目の角度：`transform.up` がワールドの上方向から反時計回りに何度回っているか（-180..180）
    pub angle: f32,
    /// 剛体の角度 `rb.rotation`（-180..180）。当たり判定はこちらで回る。普段は angle と同じだが、
    /// 左右反転したP2の Transform の回転をスクリプトで書き換えると、剛体には逆向きに入って食い違う
    /// （v1のゲームのコードでは起きない。場外からの復帰は位置だけが反映される）
    pub rot: f32,
    pub vx: f32,
    pub vy: f32,
    /// 度/秒、ワールドで反時計回りが正
    pub w: f32,
    /// `MovePlayer.groundWheelCount`
    pub ground: u32,
    pub pose: Pose,
    pub hp: f32,
    pub ult: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceError {
    pub line: usize,
    pub msg: String,
}

impl fmt::Display for TraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.msg)
    }
}

impl std::error::Error for TraceError {}

pub fn parse(text: &str) -> Result<Vec<TraceRow>, TraceError> {
    let mut lines = text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty());
    match lines.next() {
        Some((_, header)) if header.trim() == HEADER => {}
        _ => return Err(TraceError { line: 1, msg: format!("header must be `{HEADER}`") }),
    }
    lines
        .map(|(i, line)| parse_row(line).map_err(|msg| TraceError { line: i + 1, msg }))
        .collect()
}

pub fn write(rows: &[TraceRow]) -> String {
    let mut out = String::from(HEADER);
    out.push('\n');
    for r in rows {
        let pose = match r.pose {
            Pose::Normal => "Normal",
            Pose::Charge => "Charge",
            Pose::Ult => "Ult",
        };
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            r.step, r.slot, r.x, r.y, r.angle, r.rot, r.vx, r.vy, r.w, r.ground, pose, r.hp, r.ult
        ));
    }
    out
}

fn parse_row(line: &str) -> Result<TraceRow, String> {
    let f: Vec<&str> = line.trim().split(',').collect();
    if f.len() != 13 {
        return Err(format!("expected 13 columns, got {}", f.len()));
    }
    Ok(TraceRow {
        step: num(f[0], "step")?,
        slot: num(f[1], "slot")?,
        x: num(f[2], "x")?,
        y: num(f[3], "y")?,
        angle: num(f[4], "angle")?,
        rot: num(f[5], "rot")?,
        vx: num(f[6], "vx")?,
        vy: num(f[7], "vy")?,
        w: num(f[8], "w")?,
        ground: num(f[9], "ground")?,
        pose: match f[10] {
            "Normal" => Pose::Normal,
            "Charge" => Pose::Charge,
            "Ult" => Pose::Ult,
            other => return Err(format!("unknown pose `{other}`")),
        },
        hp: num(f[11], "hp")?,
        ult: num(f[12], "ult")?,
    })
}

fn num<T: FromStr>(s: &str, column: &str) -> Result<T, String> {
    s.parse().map_err(|_| format!("{column}: cannot parse `{s}`"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let rows = vec![TraceRow {
            step: 3,
            slot: 2,
            x: -4.6,
            y: -4.419_23,
            angle: 179.5,
            rot: -179.5,
            vx: 0.0,
            vy: -1.2e-5,
            w: -900.0,
            ground: 2,
            pose: Pose::Charge,
            hp: 3.0,
            ult: 0.25,
        }];
        assert_eq!(parse(&write(&rows)).unwrap(), rows);
    }

    #[test]
    fn rejects_wrong_header_and_bad_values() {
        assert!(parse("step,slot\n").is_err());
        let bad = format!("{HEADER}\n0,1,x,0,0,0,0,0,0,0,Normal,3,0\n");
        let err = parse(&bad).unwrap_err();
        assert_eq!(err.line, 2);
        assert!(err.msg.starts_with("x:"), "{err}");
    }
}
