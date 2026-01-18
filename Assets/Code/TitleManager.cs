using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.SceneManagement;

public class TitleManager : MonoBehaviour
{
    [Header("Settings")]
    [SerializeField] private string nextSceneName = "GameScene";

    void Update()
    {
        // 1. ゲーム終了
        if (ShouldQuit())
        {
            QuitGame();
        }

        // 2. Press Any Buton
        if (AnyKeyRecentlyPressd())
        {
            StartGame();
        }
    }

    private bool ShouldQuit()
    {
        var kb = Keyboard.current;
        var gp = Gamepad.current;
        var ms = Mouse.current;

        // キーボード：Escキー
        if (kb != null && kb.escapeKey.wasPressedThisFrame) return true;

        // マウス：右クリック
        if (ms != null && ms.rightButton.wasPressedThisFrame) return true;

        // ゲームパッド：〇ボタン (東側のボタン)
        if (gp != null && gp.buttonEast.wasPressedThisFrame) return true;

        return false;
    }

    void QuitGame()
    {
        Debug.Log("ゲームを終了します。");
    #if UNITY_EDITOR
        UnityEditor.EditorApplication.isPlaying = false;
    #else
        Application.Quit();
    #endif
    }

    private bool AnyKeyRecentlyPressd()
    {
        var kb = Keyboard.current;
        var gp = Gamepad.current;
        var ms = Mouse.current;

        // キーボードの何かが押された（Esc以外）
        if (kb != null && kb.anyKey.wasPressedThisFrame && !kb.escapeKey.wasPressedThisFrame) return true;
        
        // マウスのクリック
        if (ms != null && ms.leftButton.wasPressedThisFrame) return true;

        // パッドのボタンのどれか
        if (gp != null)
        {
            // パッドはanyButtonがないため、代表的なボタンをチェック
            if (gp.buttonSouth.wasPressedThisFrame ||  gp.buttonWest.wasPressedThisFrame || gp.buttonNorth.wasPressedThisFrame || gp.startButton.wasPressedThisFrame) return true;
        }

        return false;
    }

    void StartGame()
    {
        SceneManager.LoadScene(nextSceneName);
    }
}
