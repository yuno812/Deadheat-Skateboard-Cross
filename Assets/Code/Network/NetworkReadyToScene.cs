using UnityEngine;
using UnityEngine.SceneManagement;

public class NetworkReadyToScene : MonoBehaviour
{
    [SerializeField] string nextScene = "SelectScene";
    bool loaded;

    void Update()
    {
        if (loaded) return;

        // ★デバッグ用：ホスト側で条件を満たさない理由を調べる
        if (NetworkState.IsHost && Time.frameCount % 60 == 0)
        {
            // ClientConnected が true にならない限り Load() は呼ばれません
            // Debug.Log($"[CHECK] IsHost: {NetworkState.IsHost}, ClientConnected: {NetworkState.ClientConnected}");
        }

        if (NetworkState.IsHost && NetworkState.ClientConnected)
            Load();

        if (NetworkState.IsClient && NetworkState.HostFound)
            Load();
    }

    void Load()
    {
        loaded = true;
        Debug.Log("[NETWORK] Ready → Load Scene");

        // ★追加：遷移を開始したら探索コンポーネントを止める
        var hostDisc = FindFirstObjectByType<LanHostDiscovery>();
        if (hostDisc != null) hostDisc.enabled = false;
        
        var clientDisc = FindFirstObjectByType<LanClientDiscovery>();
        if (clientDisc != null) clientDisc.enabled = false;

        SceneManager.LoadScene(nextScene);
    }
}