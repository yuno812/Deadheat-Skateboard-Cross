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
        // 初期状態（スタンドアロン時）
        inputP1 = new GamepadInputProvider(0);
        inputP2 = new KeyboardInputProvider(
            Key.UpArrow, Key.DownArrow, Key.LeftArrow, Key.RightArrow,
            Key.Enter, Key.Enter, Key.Backspace, Key.Quote, Key.RightBracket
        );
    }

    // ホスト用：自分=P1(Gamepad), 相手=P2(Network)
    public void SetHostMode(NetworkInputProvider networkReceiver)
    {
        inputP1 = new GamepadInputProvider(0);
        inputP2 = networkReceiver;
        Debug.Log("[InputManager] Host Mode: P1=Gamepad, P2=Network");
    }

    // クライアント用：自分=P2(Keyboard), 相手=P1(Network)
    public void SetClientMode(NetworkInputProvider networkReceiver)
    {
        inputP1 = networkReceiver;
        inputP2 = new KeyboardInputProvider(
            Key.W, Key.S, Key.A, Key.D, // クライアント側の自分用操作
            Key.Space, Key.Space, Key.Escape, Key.Q, Key.E
        );
        Debug.Log("[InputManager] Client Mode: P1=Network, P2=Keyboard");
    }

    public void SetNetworkPlayer2(NetworkInputProvider networkInput)
    {
        inputP2 = networkInput;
    }
}