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
            // SetupDefault(); // 最初は空でもOK（選択シーンで設定するため）
        }
        else
        {
            Destroy(gameObject);
        }
    }

    public void ApplySelectedDevices(string p1Type, string p2Type)
    {
        // --- 1P の設定 ---
        if (p1Type == "Keyboard")
        {
            // 1Pがキーボードなら、相手が何であれ1Pは常にWASD
            inputP1 = CreateWASDKeyboard();
        }
        else
        {
            inputP1 = new GamepadInputProvider(0);
        }

        // --- 2P の設定 ---
        if (p2Type == "Keyboard")
        {
            // 【重要】2Pがキーボードで、かつ1Pもキーボードの時だけ「矢印キー」にする
            if (p1Type == "Keyboard")
            {
                inputP2 = CreateArrowKeyboard();
            }
            else
            {
                // 2Pだけがキーボードなら、2PもWASDを使えるようにする
                inputP2 = CreateWASDKeyboard();
            }
        }
        else
        {
            // 1Pがコントローラーなら2Pは2台目(1)、1Pがキーボードなら2Pは1台目(0)
            int p2PadIndex = (p1Type == "Controller") ? 1 : 0;
            inputP2 = new GamepadInputProvider(p2PadIndex);
        }

        Debug.Log($"[InputManager] 割り当て完了: P1={p1Type}, P2={p2Type}");
    }

    // WASD配列の生成ヘルパー
    private IInputProvider CreateWASDKeyboard()
    {
        return new KeyboardInputProvider(
            Key.W, Key.S, Key.A, Key.D, 
            Key.Space, Key.Space, Key.Escape, Key.Q, Key.E
        );
    }

    // 矢印キー配列の生成ヘルパー
    private IInputProvider CreateArrowKeyboard()
    {
        return new KeyboardInputProvider(
            Key.UpArrow, Key.DownArrow, Key.LeftArrow, Key.RightArrow,
            Key.Enter, Key.Enter, Key.Backspace, Key.L, Key.P
        );
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