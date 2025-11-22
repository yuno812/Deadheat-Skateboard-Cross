using UnityEngine;

public class PlayerSelection : MonoBehaviour
{
    public static PlayerSelection Instance;

    public GameObject playerPrefabP1;
    public GameObject heartPrefabP1;

    public GameObject playerPrefabP2;
    public GameObject heartPrefabP2;

    private void Awake()
    {
        if (Instance == null)
        {
            Instance = this;
            DontDestroyOnLoad(gameObject);
        }
        else
        {
            Destroy(gameObject);
        }
    }
}
