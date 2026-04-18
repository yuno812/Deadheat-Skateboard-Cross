using UnityEngine;
using UnityEngine.InputSystem;

public class NullInputProvider : IInputProvider
{
    private int padIndex;

    public NullInputProvider()
    {
    }

    public InputState GetInput()
    {
        InputState state = new InputState();
        return state;
    }
}
