//! Unityプロジェクトの元ファイルが、書き出し・移植の後に変わっていないかを確かめる。

use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// dsc-core が移植しているゲームのコード。移植を見直したら、`dsc check` が表示するハッシュに更新する。
/// 2026-11-04 に、CPU の入口（GroundWheelCount・RequestUltimate、CpuController を付ける処理）だけが足された版に更新した。
pub const PORTED_SOURCES: &[(&str, &str)] = &[
    ("Assets/Code/MovePlayer.cs", "d774ca0df7b509681dfcb4a28ede43f011011b3c8a2aed486d1444814f102f90"),
    ("Assets/Code/WheelGroundCheck.cs", "8ebb55b44e3d22c6d498b70bfa90c4290d49f7a680a73e824ad8fff3a6bda20e"),
    ("Assets/Code/MainCollision.cs", "74354801b7bf9cf53bb5ed70a4e260e981c963d108fbbc8342e9d2bf458e3a7b"),
    ("Assets/Code/AttackCollision.cs", "6bc54e5b59784bf53218691320b5fcf304862927187ca1dcd3a9a1621bf0cc98"),
    ("Assets/Code/GameManager.cs", "4d660bf43273837204c76a747690c48419531a1c98ec81f9d9a0b42bd0bd0b62"),
];

/// 書き出しJSONに記録された元ファイル。パスはUnityプロジェクトのルートから。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Source {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceStatus {
    Unchanged,
    Changed { current: String },
    Missing,
}

/// git の autocrlf で改行コードが変わっても同じ値になるよう、CRを除いてから計算する（Unity側と同じ）。
pub fn sha256_without_cr(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    for chunk in bytes.split(|&b| b == b'\r') {
        hasher.update(chunk);
    }
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn status(project_root: &Path, path: &str, expected_sha256: &str) -> SourceStatus {
    match std::fs::read(project_root.join(path)) {
        Ok(bytes) => {
            let current = sha256_without_cr(&bytes);
            if current == expected_sha256 {
                SourceStatus::Unchanged
            } else {
                SourceStatus::Changed { current }
            }
        }
        Err(_) => SourceStatus::Missing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_known_value() {
        assert_eq!(
            sha256_without_cr(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn carriage_returns_are_ignored() {
        assert_eq!(sha256_without_cr(b"a\r\nb\r\n"), sha256_without_cr(b"a\nb\n"));
    }
}
