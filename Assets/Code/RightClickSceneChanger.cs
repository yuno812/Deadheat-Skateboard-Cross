using UnityEngine;
using UnityEngine.SceneManagement;
using UnityEngine.InputSystem; // 新しいInput Systemを使うために必要

public class RightClickSceneChanger : MonoBehaviour
{
    [Header("Scene Settings")]
    [Tooltip("移動したいシーンの正確な名前を入力してください")]
    [SerializeField] private string targetSceneName;

    void Update()
    {
        // Mouse.current を使用して右クリックを検知します
        // rightButton.wasPressedThisFrame は押された瞬間だけ true になります
        if (Mouse.current != null && Mouse.current.rightButton.wasPressedThisFrame)
        {
            ChangeScene();
        }
    }

    /// <summary>
    /// 指定されたシーンに遷移します
    /// </summary>
    public void ChangeScene()
    {
        if (!string.IsNullOrEmpty(targetSceneName))
        {
            Debug.Log($"シーン移動を実行: {targetSceneName}");
            SceneManager.LoadScene(targetSceneName);
        }
        else
        {
            Debug.LogWarning("移動先のシーン名が設定されていません。");
        }
    }
}