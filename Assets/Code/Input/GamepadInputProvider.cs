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

        state.moveX = pad.rightTrigger.ReadValue() - pad.leftTrigger.ReadValue();

        state.rotate = -(pad.leftStick.ReadValue().x + pad.rightStick.ReadValue().x);

        state.jumpPressed = pad.buttonEast.isPressed;

        state.confirm = pad.buttonSouth.wasPressedThisFrame;

        state.cancel = pad.buttonEast.wasPressedThisFrame;

        state.skillPressed = pad.rightShoulder.wasPressedThisFrame;

        state.ultimatePressed = pad.buttonSouth.wasPressedThisFrame;
        
        return state;
    }
}
