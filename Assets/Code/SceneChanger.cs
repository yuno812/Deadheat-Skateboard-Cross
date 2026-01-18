using UnityEngine;
using UnityEngine.SceneManagement; // シーン移動に必要

public class SceneChanger : MonoBehaviour
{
    [Header("移動先のシーン名")]
    [SerializeField] private string nextSceneName;

    // ボタンから呼び出す関数
    public void ChangeScene()
    {
        if (!string.IsNullOrEmpty(nextSceneName))
        {
            Debug.Log($"{nextSceneName} へ移動します。");
            SceneManager.LoadScene(nextSceneName);
        }
        else
        {
            Debug.LogWarning("移動先のシーン名が設定されていません！");
        }
    }
}