using UnityEngine;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;

public class LanHostDiscovery : MonoBehaviour
{
    UdpClient udp;
    Thread thread;
    string myIP;

    // メインスレッドへの通知用フラグ
    bool packetReceivedFlag = false;
    string lastReceivedIP = "";
    bool clientDetectedInThread = false;
    string detectedClientIP = "";

    void Start()
    {
        myIP = GetLocalIP();
        Debug.Log("My IP is: " + myIP);

        try {
            // NetworkConfigで定義されたポート(47777)で待ち受け
            udp = new UdpClient(NetworkConfig.DISCOVERY_PORT);
            udp.EnableBroadcast = true;
            Debug.Log($"[LAN] Host listening on port {NetworkConfig.DISCOVERY_PORT}");
        } catch (System.Exception e) {
            Debug.LogError("UDP Start Error: " + e.Message);
        }

        thread = new Thread(Listen);
        thread.IsBackground = true;
        thread.Start();
    }

    void Update()
    {
        // 1. 何らかのUDPパケットが届いた際のデバッグ表示
        if (packetReceivedFlag)
        {
            packetReceivedFlag = false;
            Debug.Log($"[DEBUG] UDP Packet Arrived in Update from: {lastReceivedIP}");
        }

        // 2. 正しいDISCOVER_HOSTメッセージを処理した際のシーン遷移準備
        if (clientDetectedInThread)
        {
            clientDetectedInThread = false;
            // ネットワークの状態を接続済みに更新
            NetworkState.ClientConnected = true; 
            Debug.Log($"[LAN] Discovery response sent to {detectedClientIP}. Ready to transition.");
        }
    }

    void Listen()
    {
        IPEndPoint ep = new IPEndPoint(IPAddress.Any, 0);
        while (true)
        {
            try {
                byte[] data = udp.Receive(ref ep);
                lastReceivedIP = ep.Address.ToString();
                packetReceivedFlag = true;

                string msg = Encoding.UTF8.GetString(data);

                // ★修正：Trim() を追加して、前後の余計な空白や改行を消して判定
                if (msg.Trim().Contains("DISCOVER_HOST")) 
                {
                    if (ep.Address.ToString() == myIP || ep.Address.ToString() == "127.0.0.1") continue;

                    byte[] res = Encoding.UTF8.GetBytes("HOST_HERE");
                    udp.Send(res, res.Length, ep);

                    detectedClientIP = ep.Address.ToString();
                    clientDetectedInThread = true;
                }
            } catch { break; }
        }
    }

    string GetLocalIP()
    {
        var host = Dns.GetHostEntry(Dns.GetHostName());
        foreach (var ip in host.AddressList) {
            if (ip.AddressFamily == AddressFamily.InterNetwork) return ip.ToString();
        }
        return "127.0.0.1";
    }

    void OnDestroy() {
        if (thread != null) thread.Abort();
        udp?.Close();
    }
}