using UnityEngine;

public class ResultManager : MonoBehaviour
{
    [SerializeField] private GameObject Winner;
    [SerializeField] private GameObject Winner1;
    [SerializeField] private GameObject Winner2;
    [SerializeField] private GameObject WinnerIcon;
    [SerializeField] private GameObject WinnerNum;
    [SerializeField] private GameObject LoserIcon;
    [SerializeField] private GameObject LoserNum;
    [SerializeField] private Sprite sprite1P;
    [SerializeField] private Sprite sprite2P;

    void Start()
    {
        SpriteRenderer WinnerSprite = Winner.GetComponent<SpriteRenderer>();
        SpriteRenderer WinnerSprite1 = Winner1.GetComponent<SpriteRenderer>();
        SpriteRenderer WinnerSprite2 = Winner2.GetComponent<SpriteRenderer>();
        SpriteRenderer WinnerIconSprite = WinnerIcon.GetComponent<SpriteRenderer>();
        SpriteRenderer WinnerNumSprite = WinnerNum.GetComponent<SpriteRenderer>();
        SpriteRenderer LoserIconSprite = LoserIcon.GetComponent<SpriteRenderer>();
        SpriteRenderer LoserNumSprite = LoserNum.GetComponent<SpriteRenderer>();

        if (PlayerSelection.Instance.lose2)
        {
            Debug.Log("lose2");
            Debug.Log(PlayerSelection.Instance.lose1);
            Debug.Log(PlayerSelection.Instance.lose2);
            WinnerSprite.sprite = PlayerSelection.Instance.resultHeader1;
            WinnerSprite1.sprite = PlayerSelection.Instance.resultHeader1;
            WinnerSprite2.sprite = PlayerSelection.Instance.resultHeader1;
            WinnerIconSprite.sprite = PlayerSelection.Instance.Icon1;
            WinnerNumSprite.sprite = sprite1P;
            LoserIconSprite.sprite = PlayerSelection.Instance.Icon2;
            LoserNumSprite.sprite = sprite2P;
            UnlockCharacterSpecificAchievement();
        }
        else if (PlayerSelection.Instance.lose1)
        {
            Debug.Log("lose1");
            Debug.Log(PlayerSelection.Instance.lose1);
            Debug.Log(PlayerSelection.Instance.lose2);
            WinnerSprite.sprite = PlayerSelection.Instance.resultHeader2;
            WinnerSprite1.sprite = PlayerSelection.Instance.resultHeader2;
            WinnerSprite2.sprite = PlayerSelection.Instance.resultHeader2;
            WinnerIconSprite.sprite = PlayerSelection.Instance.Icon2;
            WinnerNumSprite.sprite = sprite2P;
            LoserIconSprite.sprite = PlayerSelection.Instance.Icon1;
            LoserNumSprite.sprite = sprite1P;
        }

        if(PlayerSelection.Instance.playerPrefabP1.name.Contains("meme") && PlayerSelection.Instance.playerPrefabP2.name.Contains("cubu") || PlayerSelection.Instance.playerPrefabP2.name.Contains("meme") && PlayerSelection.Instance.playerPrefabP1.name.Contains("cubu"))
        {
            AchievementManager.Instance.UnlockAchievement("meme_vs_cubu");
        }
    }

    private void UnlockCharacterSpecificAchievement()
    {
        string charName = PlayerSelection.Instance.playerPrefabP1.name;

        if (PlayerSelection.Instance.playerPrefabP2.name.Contains("Tutorial"))
        {
            AchievementManager.Instance.UnlockAchievement("tutorial");
        }
        else if (charName.Contains("hitsuji"))
        {
            AchievementManager.Instance.UnlockAchievement("hitsuji");
        }
        else if (charName.Contains("okuda"))
        {
            AchievementManager.Instance.UnlockAchievement("okuda");
        }
    }
}
