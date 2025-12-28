using UnityEngine;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;

public class LanClientDiscovery : MonoBehaviour
{
    public string foundHostIP { get; private set; }
    UdpClient udp;
    Thread thread;
    bool running;
    bool hostDetectedInThread = false;

    void Start()
    {
        // 既存の通信との衝突を避けるための設定
        udp = new UdpClient();
        udp.EnableBroadcast = true;
        udp.Client.ReceiveTimeout = 1000;
        
        running = true;
        thread = new Thread(Discover);
        thread.IsBackground = true;
        thread.Start();
    }

    void Update()
    {
        if (hostDetectedInThread)
        {
            hostDetectedInThread = false;
            NetworkState.HostFound = true;
            Debug.Log($"[LAN] Host found and connected: {foundHostIP}");

            // ★重要：見つけたIPを送信機(Sender)に自動セットする
            var sender = GetComponent<InputSyncSender>();
            if (sender != null)
            {
                sender.SetRemoteIP(foundHostIP);
            }
        }
    }

    void Discover()
    {
        // 全員宛（255.255.255.255）の窓口を設定
        IPEndPoint broadcastEP = new IPEndPoint(IPAddress.Broadcast, NetworkConfig.DISCOVERY_PORT);
        byte[] msg = Encoding.UTF8.GetBytes("DISCOVER_HOST");

        while (running && string.IsNullOrEmpty(foundHostIP))
        {
            try {
                // 「誰かホストはいませんか？」とネットワーク全体に叫ぶ
                udp.Send(msg, msg.Length, broadcastEP);

                // ホストからの返信を待つ
                IPEndPoint sender = new IPEndPoint(IPAddress.Any, 0);
                byte[] data = udp.Receive(ref sender);
                string res = Encoding.UTF8.GetString(data);

                // ホストが「ここにいるよ」と返してきたらIPを記録
                if (res.Trim().Contains("HOST_HERE")) {
                    foundHostIP = sender.Address.ToString();
                    hostDetectedInThread = true;
                    running = false; // 発見したのでループ終了
                }
            } catch (SocketException) {
                // タイムアウト時はここに来るが、ループを続ける
            }
            Thread.Sleep(1000); // 1秒おきに再試行
        }
    }

    void OnDestroy() {
        running = false;
        if (thread != null) thread.Abort();
        udp?.Close();
    }
}