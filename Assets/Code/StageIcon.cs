using UnityEngine;
using System.Collections;

public class StageIcon : MonoBehaviour
{
    [Header("スプライト")]
    [SerializeField] private Sprite normalSprite;
    [SerializeField] private Sprite selectedSprite;
    [SerializeField] private Sprite headerSprite;

    [Header("ステージ設定")]
    [SerializeField] private string sceneName;

    [Header("ヘッダー")]
    [SerializeField] private GameObject headerObject;

    [Header("スライド設定")]
    [SerializeField] private float slideDistance = 0.3f;
    [SerializeField] private float slideDuration = 0.2f;

    private SpriteRenderer spriteRenderer;
    private SpriteRenderer headerRenderer;

    private Vector3 headerOriginalPos;
    private Coroutine slideCoroutine;
    private bool headerShown = false;

    public string SceneName => sceneName;

    void Awake()
    {
        spriteRenderer = GetComponent<SpriteRenderer>();

        if (headerObject != null)
        {
            headerRenderer = headerObject.GetComponent<SpriteRenderer>();
            headerOriginalPos = headerObject.transform.position;
        }
    }

    public void SetSelected(bool selected)
    {
        if (spriteRenderer == null) return;

        spriteRenderer.sprite = selected ? selectedSprite : normalSprite;

        if (selected && headerRenderer != null)
        {
            headerRenderer.sprite = headerSprite;

            if (!headerShown)
            {
                if (slideCoroutine != null) StopCoroutine(slideCoroutine);
                slideCoroutine = StartCoroutine(SlideIn(headerObject.transform));
                headerShown = true;
            }
        }
        else
        {
            headerShown = false;
        }
    }

    private IEnumerator SlideIn(Transform target)
    {
        Vector3 startPos = headerOriginalPos + Vector3.left * slideDistance;
        target.position = startPos;

        float elapsed = 0f;
        while (elapsed < slideDuration)
        {
            target.position = Vector3.Lerp(startPos, headerOriginalPos, elapsed / slideDuration);
            elapsed += Time.deltaTime;
            yield return null;
        }
        target.position = headerOriginalPos;
    }
}
