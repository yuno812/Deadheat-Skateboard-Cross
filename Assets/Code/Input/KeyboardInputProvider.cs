using UnityEngine;
using UnityEngine.InputSystem;

public class KeyboardInputProvider : IInputProvider
{
    private Key up;
    private Key down;
    private Key left;
    private Key right;
    private Key jump;
    private Key confirm;
    private Key cancel;

    public KeyboardInputProvider(
        Key up,
        Key down,
        Key left,
        Key right,
        Key jump,
        Key confirm,
        Key cancel
    )
    {
        this.up = up;
        this.down = down;
        this.left = left;
        this.right = right;
        this.jump = jump;
        this.confirm = confirm;
        this.cancel = cancel;
    }

    public InputState GetInput()
    {
        var keyboard = Keyboard.current;
        InputState state = new InputState();

        if (keyboard == null) return state;

        // ===== UI / 選択用（1回押し）=====
        if (keyboard[up].wasPressedThisFrame) state.move.y = 1;
        if (keyboard[down].wasPressedThisFrame) state.move.y = -1;
        if (keyboard[left].wasPressedThisFrame) state.move.x = -1;
        if (keyboard[right].wasPressedThisFrame) state.move.x = 1;

        // ===== ゲーム用（押しっぱなし）=====
        float moveX = 0;
        if (keyboard[down].isPressed) moveX -= 1;
        if (keyboard[up].isPressed) moveX += 1;

        float rotate = 0;
        if (keyboard[left].isPressed) rotate += 1;
        if (keyboard[right].isPressed) rotate -= 1;

        state.moveX = moveX;
        state.rotate = rotate;

        // ===== アクション =====
        state.jumpPressed = keyboard[jump].isPressed;
        state.confirm = keyboard[confirm].wasPressedThisFrame;
        state.cancel = keyboard[cancel].wasPressedThisFrame;

        return state;
    }
}
