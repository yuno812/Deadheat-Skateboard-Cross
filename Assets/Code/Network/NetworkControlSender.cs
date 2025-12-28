using UnityEngine;
using System.Net;
using System.Net.Sockets;
using System.Text;

public class NetworkControlSender : MonoBehaviour
{
    UdpClient udp = new UdpClient();
    IPEndPoint ep;

    public void SetClientIP(string ip)
    {
        ep = new IPEndPoint(IPAddress.Parse(ip), NetworkConfig.CONTROL_PORT);
    }

    public void SendLoadScene(string scene)
    {
        if (!NetworkRole.IsHost) return;

        var msg = new ControlMessage { command = "LOAD_SCENE", value = scene };
        string json = JsonUtility.ToJson(msg);
        udp.Send(Encoding.UTF8.GetBytes(json), json.Length, ep);
    }
}
