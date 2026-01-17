using UnityEngine;

public struct InputState
{
    public Vector2 move;       // UI / 選択用（上下左右）
    public float moveX;        // ゲーム中移動用（左右）
    public float rotate;       // 回転
    public bool jumpPressed;
    public bool confirm;
    public bool cancel;
    public bool skillPressed;  // 固有スキル
    public bool ultimatePressed; // ウルト
}

