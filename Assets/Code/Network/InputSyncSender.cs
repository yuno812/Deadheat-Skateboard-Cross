using UnityEngine;
using System.Net.Sockets;
using System.Text;

public class InputSyncSender : MonoBehaviour
{
    UdpClient udp;
    string remoteIP;
    float discoveryTimer = 0;

    void Start()
    {
        udp = new UdpClient();
        udp.EnableBroadcast = true;
    }

    public void SetRemoteIP(string ip)
    {
        remoteIP = ip;
        if (string.IsNullOrEmpty(ip)) Debug.Log("[NETWORK] Sender: Broadcast Mode");
        else Debug.Log($"[NETWORK] Sender: Target set to {remoteIP}");
    }

    void Update()
    {
        if (InputManager.Instance == null) return;

        // 自分がホストなら「自分のP1」を、クライアントなら「自分のP2」を送信する
        InputState state = NetworkState.IsHost ? 
            InputManager.Instance.inputP1.GetInput() : 
            InputManager.Instance.inputP2.GetInput();

        string json = JsonUtility.ToJson(state);
        byte[] data = Encoding.UTF8.GetBytes(json);

        try
        {
            string target = string.IsNullOrEmpty(remoteIP) ? "255.255.255.255" : remoteIP;
            udp.Send(data, data.Length, target, NetworkConfig.INPUT_PORT);
        }
        catch { }

        // クライアントのみホスト探索パケットを送る
        if (NetworkState.IsClient)
        {
            discoveryTimer += Time.deltaTime;
            if (discoveryTimer >= 1.0f) {
                discoveryTimer = 0;
                SendDiscoveryMessage();
            }
        }
    }

    void SendDiscoveryMessage()
    {
        byte[] msg = Encoding.UTF8.GetBytes("DISCOVER_HOST");
        try {
            string target = string.IsNullOrEmpty(remoteIP) ? "255.255.255.255" : remoteIP;
            udp.Send(msg, msg.Length, target, NetworkConfig.DISCOVERY_PORT);
        } catch { }
    }

    void OnDestroy() { udp?.Close(); }
}