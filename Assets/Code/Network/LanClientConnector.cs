using UnityEngine;

public class LanClientConnector : MonoBehaviour
{
    // [SerializeField] を削除して、シリアライズエラーを回避
    private LanClientDiscovery discovery;
    private bool connected;

    void Start()
    {
        // 実行時に自分自身（または同じオブジェクト）から取得
        discovery = GetComponent<LanClientDiscovery>();
    }

    void Update()
    {
        if (connected) return;

        // クライアント側：ホストを見つけたら送信先をセット
        if (NetworkState.IsClient && NetworkState.HostFound && discovery != null)
        {
            if (!string.IsNullOrEmpty(discovery.foundHostIP))
            {
                ApplyRemoteIP(discovery.foundHostIP);
            }
        }

        // ホスト側：クライアントが接続してきたら確定
        if (NetworkState.IsHost && NetworkState.ClientConnected)
        {
            connected = true;
            Debug.Log("[LAN] Host: Client Connection Confirmed");
        }
    }

    void ApplyRemoteIP(string ip)
    {
        connected = true;
        Debug.Log("[LAN] Switching P2 input to Network");
        
        var sender = GetComponent<InputSyncSender>();
        if (sender != null) sender.SetRemoteIP(ip);
    }
}