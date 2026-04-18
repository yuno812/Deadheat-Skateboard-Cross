using UnityEngine;
using System.Collections;
using UnityEngine.InputSystem;
using UnityEngine.SceneManagement;

public class RetrunResult : MonoBehaviour
{
    [SerializeField] private GameObject press;
    // 1秒後に true になるフラグ
    public bool isFinished = false;

    void Start()
    {
        // タイマーの開始
        StartCoroutine(StartTimer());
        press.SetActive(false);
    }

    private IEnumerator StartTimer()
    {
        // 1秒間待機
        yield return new WaitForSeconds(1f);

        // フラグを true に変更
        isFinished = true;
        press.SetActive(true);
    }
    
    void Update()
    {
        if (isFinished && AnyKeyRecentlyPressd())
        {
            PlayerSelection.Instance.lose1 = false;
            PlayerSelection.Instance.lose2 = false;
            SceneManager.LoadScene("MenuScene");
        }
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
}
