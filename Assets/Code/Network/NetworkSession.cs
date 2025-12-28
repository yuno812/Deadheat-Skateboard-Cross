using UnityEngine;

public class NetworkSession : MonoBehaviour
{
    public static NetworkSession Instance;

    public bool isHost;
    public string remoteIP;

    void Awake()
    {
        if (Instance != null)
        {
            Destroy(gameObject);
            return;
        }
        Instance = this;
        DontDestroyOnLoad(gameObject);
    }

    public void StartAsHost()
    {
        isHost = true;
        Debug.Log("Started as HOST");
    }

    public void StartAsClient(string hostIP)
    {
        isHost = false;
        remoteIP = hostIP;
        Debug.Log("Started as CLIENT → " + hostIP);
    }
}
