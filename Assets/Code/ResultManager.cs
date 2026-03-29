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
        Sprite WinnerSprite = Winner.GetComponent<SpriteRenderer>().sprite;
        Sprite WinnerSprite1 = Winner1.GetComponent<SpriteRenderer>().sprite;
        Sprite WinnerSprite2 = Winner2.GetComponent<SpriteRenderer>().sprite;
        Sprite WinnerIconSprite = WinnerIcon.GetComponent<SpriteRenderer>().sprite;
        Sprite WinnerNumSprite = WinnerNum.GetComponent<SpriteRenderer>().sprite;
        Sprite LoserIconSprite = LoserIcon.GetComponent<SpriteRenderer>().sprite;
        Sprite LoserNumSprite = LoserNum.GetComponent<SpriteRenderer>().sprite;

        if (PlayerSelection.Instance.lose2)
        {
            WinnerSprite = PlayerSelection.Instance.resultHeader1;
            WinnerSprite1 = PlayerSelection.Instance.resultHeader1;
            WinnerSprite2 = PlayerSelection.Instance.resultHeader1;
            WinnerIconSprite = PlayerSelection.Instance.Icon1;
            WinnerNumSprite = sprite1P;
            LoserIconSprite = PlayerSelection.Instance.Icon2;
            LoserNumSprite = sprite2P;
        }
        else if (PlayerSelection.Instance.lose1)
        {
            WinnerSprite = PlayerSelection.Instance.resultHeader2;
            WinnerSprite1 = PlayerSelection.Instance.resultHeader2;
            WinnerSprite2 = PlayerSelection.Instance.resultHeader2;
            WinnerIconSprite = PlayerSelection.Instance.Icon2;
            WinnerNumSprite = sprite2P;
            LoserIconSprite = PlayerSelection.Instance.Icon1;
            LoserNumSprite = sprite1P;
        }
    }
}
