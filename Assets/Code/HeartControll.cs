using UnityEngine;
using UnityEngine.UI;

public class HeartControll : MonoBehaviour
{
    [SerializeField] private Texture fullHeart;
    [SerializeField] private Texture halfHeart;
    [SerializeField] private Texture nullHeart;

    private RawImage image;

    void Awake()
    {
        image = GetComponent<RawImage>();
    }

    public void SetFull() => image.texture = fullHeart;
    public void SetHalf() => image.texture = halfHeart;
    public void SetNull() => image.texture = nullHeart;
}
