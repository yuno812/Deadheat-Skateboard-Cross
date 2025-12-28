using UnityEngine;
using UnityEngine.InputSystem;
using System.Collections;

public class AutoNetworkBootstrap : MonoBehaviour
{
    IEnumerator Start()
    {
        Debug.Log("--- [NETWORK BOOTSTRAP] ---");
        float timer = 0;
        while (timer < 3f)
        {
            if (Keyboard.current != null)
            {
                if (Keyboard.current.hKey.wasPressedThisFrame) { BecomeHost(); yield break; }
                if (Keyboard.current.cKey.wasPressedThisFrame) 
                {
                    var disc = GetComponent<LanClientDiscovery>();
                    string targetIP = (disc != null && !string.IsNullOrEmpty(disc.foundHostIP)) ? disc.foundHostIP : "";
                    BecomeClient(targetIP); 
                    yield break; 
                }
            }
            timer += Time.deltaTime;
            yield return null;
        }
        BecomeHost();
    }

    void BecomeHost()
    {
        NetworkState.IsHost = true;
        NetworkState.IsClient = false;

        // 相手(P2)の入力を受け取る準備
        var receiver = gameObject.AddComponent<InputSyncReceiver>();
        InputManager.Instance.SetHostMode(receiver);

        gameObject.AddComponent<InputSyncSender>();
        gameObject.AddComponent<LanHostDiscovery>();
        gameObject.AddComponent<LanClientConnector>(); 
    }

    void BecomeClient(string ip)
    {
        NetworkState.IsHost = false;
        NetworkState.IsClient = true;
        NetworkState.HostFound = true;

        // 相手(P1)の入力を受け取る準備
        var receiver = gameObject.AddComponent<InputSyncReceiver>();
        InputManager.Instance.SetClientMode(receiver);

        var sender = gameObject.AddComponent<InputSyncSender>();
        sender.SetRemoteIP(ip); // 自動取得したIPをセット

        gameObject.AddComponent<LanClientDiscovery>();
        gameObject.AddComponent<LanClientConnector>();
    }
}