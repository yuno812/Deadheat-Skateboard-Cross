//! 書き出しJSONからワールドを組み立て、`MovePlayer` の移植を含めて1ステップ進める。
//! グローバルな状態は持たない。（物理とステップ処理はM2で実装）

pub mod export;
pub mod source;
