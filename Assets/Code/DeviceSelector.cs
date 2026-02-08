using UnityEngine;
using UnityEngine.UI;
using UnityEngine.EventSystems;

public class DeviceSelector : MonoBehaviour, IPointerEnterHandler, IPointerExitHandler, IPointerClickHandler
{
    public enum DeviceType { Keyboard, Controller }
    public enum PlayerNumber { P1, P2 }

    [Header("設定")]
    public PlayerNumber playerNumber;
    public DeviceType deviceType;
    [Header("相方")]
    [SerializeField] private DeviceSelector partner;
    [Header("イラスト (Sprite)")]
    [SerializeField] private Sprite normalSprite;   // 通常時
    [SerializeField] private Sprite hoverSprite;    // マウスが乗った時
    [SerializeField] private Sprite selectedSprite; // 決定した時

    private Image targetImage;
    public bool isSelected = false;

    void Awake()
    {
        targetImage = GetComponent<Image>();
        targetImage.sprite = normalSprite;
    }

    // マウスが乗ったとき
    public void OnPointerEnter(PointerEventData eventData)
    {
        if (isSelected) return;
        targetImage.sprite = hoverSprite;
    }

    // マウスが離れたとき
    public void OnPointerExit(PointerEventData eventData)
    {
        if (isSelected) return;
        targetImage.sprite = normalSprite;
    }

    // マウスでクリックしたとき
    public void OnPointerClick(PointerEventData eventData)
    {
        SelectThisDevice();
    }

    public void SelectThisDevice()
    {
        if (isSelected) return;
        isSelected = true;
        targetImage.sprite = selectedSprite;
        if (partner != null) partner.Deselect();

        // 親（Manager）に選択が終わったことを通知
        Object.FindAnyObjectByType<DeviceSelectionManager>().CheckAllPlayersReady();
    }

    // 他が選ばれた時に状態を戻す用
    public void Deselect()
    {
        isSelected = false;
        targetImage.sprite = normalSprite;
    }
}