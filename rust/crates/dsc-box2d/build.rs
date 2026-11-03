// Box2D 2.4.1（box2d/、MIT ライセンス）と仲介層（src/shim.cpp）をビルドする。

use std::path::Path;

fn main() {
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .include("box2d/include")
        .include("box2d/src")
        .file("src/shim.cpp")
        // shim.cpp のコメントは UTF-8。指定しないと日本語環境の MSVC は Shift-JIS として読み、構文が壊れる
        .flag_if_supported("/utf-8")
        .warnings(false);
    // b2Assert を外す（潰れた多角形などは Rust 側で事前に除いている）
    if std::env::var("PROFILE").as_deref() == Ok("release") {
        build.define("NDEBUG", None);
    }
    add_sources(&mut build, Path::new("box2d/src"));
    build.compile("box2d");

    println!("cargo:rerun-if-changed=src/shim.cpp");
    println!("cargo:rerun-if-changed=box2d");
}

fn add_sources(build: &mut cc::Build, dir: &Path) {
    for entry in std::fs::read_dir(dir).expect("box2d/src") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            add_sources(build, &path);
        } else if path.extension().is_some_and(|e| e == "cpp") {
            build.file(path);
        }
    }
}
