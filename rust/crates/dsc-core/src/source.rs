//! Unityプロジェクトの元ファイルが、書き出し・移植の後に変わっていないかを確かめる。

use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// dsc-core が移植しているゲームのコード。移植を見直したら、`dsc check` が表示するハッシュに更新する。
/// 今の値は仕様書（2026-11-02）が前提にしている版。
pub const PORTED_SOURCES: &[(&str, &str)] = &[
    ("Assets/Code/MovePlayer.cs", "529b04d0ecfba959d68a516f0a61bc98fd3f57c5df3a144043553412d4a5c50a"),
    ("Assets/Code/WheelGroundCheck.cs", "8ebb55b44e3d22c6d498b70bfa90c4290d49f7a680a73e824ad8fff3a6bda20e"),
    ("Assets/Code/MainCollision.cs", "74354801b7bf9cf53bb5ed70a4e260e981c963d108fbbc8342e9d2bf458e3a7b"),
    ("Assets/Code/AttackCollision.cs", "6bc54e5b59784bf53218691320b5fcf304862927187ca1dcd3a9a1621bf0cc98"),
    ("Assets/Code/GameManager.cs", "0349dad9cf9a5e8f13d556952d2c7a2acb7d6ff96743d6580395643ee6305843"),
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
