using UnityEngine;
using UnityEngine.InputSystem;

public class GamepadInputProvider : IInputProvider
{
    private int padIndex;

    public GamepadInputProvider(int index)
    {
        padIndex = index;
    }

    public InputState GetInput()
    {
        InputState state = new InputState();

        if (Gamepad.all.Count <= padIndex) return state;

        var pad = Gamepad.all[padIndex];
        if (pad == null) return state;

        Vector2 stick = pad.leftStick.ReadValue();

        state.move = new Vector2(Mathf.Round(stick.x), Mathf.Round(stick.y));

        state.moveX = pad.leftStick.ReadValue().y;

        state.rotate = -pad.rightStick.ReadValue().x;

        state.jumpPressed = pad.buttonSouth.isPressed;

        state.confirm = pad.buttonSouth.wasPressedThisFrame;

        state.cancel = pad.buttonEast.wasPressedThisFrame;

        state.skillPressed = pad.buttonWest.wasPressedThisFrame;

        state.ultimatePressed = pad.buttonNorth.wasPressedThisFrame;
        
        return state;
    }
}
