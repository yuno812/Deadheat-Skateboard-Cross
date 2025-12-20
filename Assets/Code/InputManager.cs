using UnityEngine;
using UnityEngine.InputSystem;

public class InputManager : MonoBehaviour
{
    public static InputManager Instance;

    public IInputProvider inputP1;
    public IInputProvider inputP2;

    void Awake()
    {
        if (Instance == null)
        {
            Instance = this;
            DontDestroyOnLoad(gameObject);
            SetupDefault();
        }
        else
        {
            Destroy(gameObject);
        }
    }

    void SetupDefault()
    {
        inputP1 = new KeyboardInputProvider(
            Key.W, Key.S, Key.A, Key.D,
            Key.Space,     // jump
            Key.Space,     // confirm
            Key.Escape     // cancel
        );

        inputP2 = new KeyboardInputProvider(
            Key.UpArrow, Key.DownArrow, Key.LeftArrow, Key.RightArrow,
            Key.Enter,   // jump
            Key.Enter,   // confirm
            Key.Backspace // cancel
        );
    }

}
