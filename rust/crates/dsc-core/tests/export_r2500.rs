//! リポジトリに入っている R2500 の書き出しを読めるか、中身が前提どおりかを確かめる。

use std::path::Path;

use dsc_core::export::{Pose, ShapeKind, StageDir};

fn r2500() -> StageDir {
    StageDir::new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"), "R2500")
}

#[test]
fn stage_loads() {
    let stage = r2500().load_stage().unwrap();
    assert!((stage.world.dt - 0.02).abs() < 1e-6);
    assert_eq!(stage.world.gravity, [0.0, -9.81]);
    assert!(!stage.shapes.is_empty());
    assert!(stage.shapes.iter().all(|s| s.tag == "Ground" && s.kind == ShapeKind::Polygon));
}

#[test]
fn all_selectable_characters_are_exported() {
    let names = r2500().characters().unwrap();
    assert_eq!(names, ["cubu", "hitsuji", "maiden", "meme", "nagisa", "okuda", "zero"]);
}

#[test]
fn characters_have_both_slots_and_all_poses() {
    let dir = r2500();
    for name in dir.characters().unwrap() {
        let c = dir.load_character(&name).unwrap();
        let p1 = c.slot(1).unwrap();
        let p2 = c.slot(2).unwrap();
        assert!(!p1.mirrored && p2.mirrored, "{name}");

        for slot in [p1, p2] {
            assert!(slot.body.mass > 0.0, "{name}");
            for pose in [Pose::Normal, Pose::Charge, Pose::Ult] {
                assert!(slot.poses[&pose].inertia > 0.0, "{name} {pose:?}");
                let shapes: Vec<_> = slot.shapes.iter().filter(|s| s.pose == pose).collect();
                for tag in ["TireCollision", "MainCollision", "AttackCollision"] {
                    assert!(shapes.iter().any(|s| s.tag == tag), "{name} {pose:?} has no {tag}");
                }
                assert!(
                    shapes
                        .iter()
                        .filter(|s| s.tag == "TireCollision")
                        .all(|s| s.handlers.iter().any(|h| h == "WheelGroundCheck")),
                    "{name} {pose:?}"
                );
            }
        }

        // P2は左右反転しているので、重心のxは符号が逆になる
        let com1 = p1.poses[&Pose::Normal].center_of_mass;
        let com2 = p2.poses[&Pose::Normal].center_of_mass;
        assert!((com1[0] + com2[0]).abs() < 1e-3, "{name}: {com1:?} vs {com2:?}");
        assert!((com1[1] - com2[1]).abs() < 1e-3, "{name}: {com1:?} vs {com2:?}");
    }
}
