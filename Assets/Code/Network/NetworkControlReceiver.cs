using UnityEngine;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;
using UnityEngine.SceneManagement;

public class NetworkControlReceiver : MonoBehaviour
{
    UdpClient udp;
    Thread thread;

    void Start()
    {
        udp = new UdpClient(NetworkConfig.CONTROL_PORT);
        thread = new Thread(Listen);
        thread.IsBackground = true;
        thread.Start();
    }

    void Listen()
    {
        IPEndPoint ep = new IPEndPoint(IPAddress.Any, 0);

        while (true)
        {
            var json = Encoding.UTF8.GetString(udp.Receive(ref ep));
            var msg = JsonUtility.FromJson<ControlMessage>(json);

            if (msg.command == "LOAD_SCENE")
                UnityMainThreadDispatcher.Enqueue(() => SceneManager.LoadScene(msg.value));
        }
    }
}
