using UnityEngine;

public class SelectSceneInitializer : MonoBehaviour
{
    void Awake()
    {
        if (PlayerSelection.Instance == null) return;

        // ステージ選択からやり直す
        PlayerSelection.Instance.stageselect = false;
        PlayerSelection.Instance.nextSceneName = "";

        // キャラ選択状態もリセット
        PlayerSelection.Instance.playerPrefabP1 = null;
        PlayerSelection.Instance.playerPrefabP2 = null;
        PlayerSelection.Instance.heartPrefabP1 = null;
        PlayerSelection.Instance.heartPrefabP2 = null;
    }
}
