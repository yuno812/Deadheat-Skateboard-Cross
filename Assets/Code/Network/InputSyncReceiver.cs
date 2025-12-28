using UnityEngine;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;

// NetworkInputProvider を継承
public class InputSyncReceiver : NetworkInputProvider 
{
    UdpClient udp;
    Thread thread;

    // もし親クラスで定義されていない場合のエラーを防ぐため、
    // ここで変数を明示的に管理します（継承元にある場合は virtual/override の調整が必要ですが、一旦これで通るはずです）
    protected InputState latestState;
    protected readonly object lockObj = new object();

    void Start()
    {
        try {
            // 他のアプリとポートを共有できるように設定
            udp = new UdpClient();
            udp.Client.SetSocketOption(SocketOptionLevel.Socket, SocketOptionName.ReuseAddress, true);
            udp.Client.Bind(new IPEndPoint(IPAddress.Any, NetworkConfig.INPUT_PORT));
            
            thread = new Thread(Listen);
            thread.IsBackground = true;
            thread.Start();
            Debug.Log($"[NETWORK] Receiver started on port {NetworkConfig.INPUT_PORT}");
        } catch (System.Exception e) {
            Debug.LogError($"[NETWORK] Receiver Port Error: {e.Message}");
        }
    }

    void Listen()
    {
        IPEndPoint ep = new IPEndPoint(IPAddress.Any, 0);
        string myIP = GetLocalIP();

        while (true)
        {
            try {
                byte[] data = udp.Receive(ref ep);
                
                // 送信元が自分（自PC）なら処理をスキップ
                if (ep.Address.ToString() == myIP || ep.Address.ToString() == "127.0.0.1") 
                {
                    continue; 
                }

                string json = Encoding.UTF8.GetString(data);
                lock (lockObj) {
                    latestState = JsonUtility.FromJson<InputState>(json);
                }
            } catch { break; }
        }
    }

    // InputManagerから呼ばれる入力取得メソッド
    public override InputState GetInput()
    {
        lock (lockObj) {
            return latestState;
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

    void OnDestroy()
    {
        if (thread != null) thread.Abort();
        if (udp != null) udp.Close();
    }
}