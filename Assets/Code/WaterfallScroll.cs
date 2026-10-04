using UnityEngine;
using UnityEngine.UI;

public class WaterfallScroll : MonoBehaviour
{
    [SerializeField] private float scrollSpeed = 0.2f;

    private RawImage waterfall;
    private Rect uv;

    void Awake()
    {
        waterfall = GetComponent<RawImage>();
        uv = waterfall.uvRect;
    }

    void Update()
    {
        // UV座標をずらして、滝が下に流れるように見せる
        uv.y += scrollSpeed * Time.deltaTime;

        // UV座標が大きくなりすぎないようにする
        uv.y = Mathf.Repeat(uv.y, 1f);

        waterfall.uvRect = uv;
    }
}