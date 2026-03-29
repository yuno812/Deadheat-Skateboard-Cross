using UnityEngine;

public class PlayerSelection : MonoBehaviour
{
    public static PlayerSelection Instance;

    public GameObject playerPrefabP1;
    public GameObject heartPrefabP1;
    public Sprite resultHeader1;
    public Sprite Icon1;
    public bool lose1 = false;

    public GameObject playerPrefabP2;
    public GameObject heartPrefabP2;
    public Sprite resultHeader2;
    public Sprite Icon2;
    public bool lose2 = false;

    public bool stageselect = false;
    public string nextSceneName;

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
