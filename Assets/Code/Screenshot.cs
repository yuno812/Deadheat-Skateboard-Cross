using UnityEngine;
using System.IO;
#if ENABLE_INPUT_SYSTEM
using UnityEngine.InputSystem; // 新しいInput Systemを使用する場合に必要
#endif

/// <summary>
/// 指定されたキー（デフォルトはSpaceキー）が押されたときにゲーム画面のスクリーンショットを撮影・保存します。
/// </summary>
public class Screenshot : MonoBehaviour
{
    [Header("Settings")]
    [Tooltip("スクリーンショット保存先フォルダ名（プロジェクトルートフォルダからの相対パス）")]
    [SerializeField] private string folderName = "Screenshots";

    [Tooltip("保存時のファイル名の接頭辞")]
    [SerializeField] private string filePrefix = "Screenshot_";

    void Update()
    {
        bool isKeyPressed = false;

#if ENABLE_INPUT_SYSTEM
        // 新しいInput Systemを使用している場合
        if (Keyboard.current != null && Keyboard.current.spaceKey.wasPressedThisFrame)
        {
            isKeyPressed = true;
        }
#else
        // 従来のInput Managerを使用している場合
        if (Input.GetKeyDown(KeyCode.Space))
        {
            isKeyPressed = true;
        }
#endif

        if (isKeyPressed)
        {
            TakeScreenshot();
        }
    }

    /// <summary>
    /// スクリーンショットを撮影し、ファイルに保存します。
    /// </summary>
    public void TakeScreenshot()
    {
        // 保存先フォルダの絶対パスを取得
        string directoryPath = Path.Combine(Directory.GetCurrentDirectory(), folderName);

        // フォルダが存在しない場合は作成
        if (!Directory.Exists(directoryPath))
        {
            Directory.CreateDirectory(directoryPath);
        }

        // 現在の時刻を取得してユニークなファイル名を作成 (例: Screenshot_20260523_141830.png)
        string timestamp = System.DateTime.Now.ToString("yyyyMMdd_HHmmss");
        string fileName = $"{filePrefix}{timestamp}.png";
        string fullPath = Path.Combine(directoryPath, fileName);

        // スクリーンショットを撮影して保存
        ScreenCapture.CaptureScreenshot(fullPath);

        Debug.Log($"[ScreenshotTaker] スクリーンショットを保存しました: {fullPath}");
    }
}