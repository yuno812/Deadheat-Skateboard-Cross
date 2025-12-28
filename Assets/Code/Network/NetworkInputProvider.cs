using UnityEngine;

// MonoBehaviour を継承させる
public class NetworkInputProvider : MonoBehaviour, IInputProvider
{
    protected InputState currentInput = new InputState();

    public virtual InputState GetInput()
    {
        return currentInput;
    }

    public virtual void Apply(InputState state)
    {
        currentInput = state;
    }
}