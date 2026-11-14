use std::path::{Path, PathBuf};

/// Unityプロジェクトのルート。カレントディレクトリから上へ ProjectSettings/ProjectVersion.txt を探し、
/// 見つからなければビルドしたときのリポジトリを使う。
pub fn project_root() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir()
        && let Some(dir) = cwd.ancestors().find(|d| is_project_root(d))
    {
        return dir.to_owned();
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|d| is_project_root(d))
        .expect("dsc-cli is built inside the Unity project")
        .to_owned()
}

/// 書き出し・シナリオ・トレースの置き場所。環境変数 DSC_DATA があればそちら
pub fn data_dir(project_root: &Path) -> PathBuf {
    match std::env::var_os("DSC_DATA") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => project_root.join("rust").join("data"),
    }
}

fn is_project_root(dir: &Path) -> bool {
    dir.join("ProjectSettings").join("ProjectVersion.txt").is_file()
}
