using UnityEngine;
using System.Collections;

public class CharacterIcon : MonoBehaviour
{
    [Header("キャラクター本体")]
    public GameObject PlayerPrefab;
    public GameObject HeartPrefab;

    [Header("スプライト")]
    [SerializeField] private Sprite normalSprite;
    [SerializeField] private Sprite p1SelectedSprite;
    [SerializeField] private Sprite p2SelectedSprite;
    [SerializeField] private Sprite bothSelectedSprite;
    [SerializeField] private Sprite headerSprite;

    [Header("追加オブジェクト")]
    [SerializeField] private GameObject p1Object;
    [SerializeField] private GameObject p2Object;

    [Header("スライド設定")]
    [SerializeField] private float slideDistance = 0.3f;
    [SerializeField] private float slideDuration = 0.2f;

    [Header("ステータス値の設定")]
    [SerializeField] private int speedCount;
    [SerializeField] private int physicalCount;
    [SerializeField] private int specialCount;

    [Header("1P用の表示先")]
    [SerializeField] private StatBarSpawner p1SpeedSpawner;
    [SerializeField] private StatBarSpawner p1PhysicalSpawner;
    [SerializeField] private StatBarSpawner p1SpecialSpawner;

    [Header("2P用の表示先")]
    [SerializeField] private StatBarSpawner p2SpeedSpawner;
    [SerializeField] private StatBarSpawner p2PhysicalSpawner;
    [SerializeField] private StatBarSpawner p2SpecialSpawner;

    private SpriteRenderer spriteRenderer;
    private SpriteRenderer p1sprite;
    private SpriteRenderer p2sprite;

    private Vector3 p1OriginalPos;
    private Vector3 p2OriginalPos;

    private Coroutine p1SlideCoroutine;
    private Coroutine p2SlideCoroutine;

    private bool p1HeaderShown = false;
    private bool p2HeaderShown = false;

    void Awake()
    {
        spriteRenderer = GetComponent<SpriteRenderer>();
        if (p1Object != null) p1sprite = p1Object.GetComponent<SpriteRenderer>();
        if (p2Object != null) p2sprite = p2Object.GetComponent<SpriteRenderer>();

        UpdateOriginalPositions();

        if (spriteRenderer == null)
            Debug.LogError("SpriteRenderer がアタッチされていません！");
    }

    // ★ 現在位置を基準として保存
    void UpdateOriginalPositions()
    {
        if (p1Object != null) p1OriginalPos = p1Object.transform.position;
        if (p2Object != null) p2OriginalPos = p2Object.transform.position;
    }

    public void SetSelected(bool p1Cursor, bool p2Cursor, bool confirmedP1, bool confirmedP2)
    {
        if (spriteRenderer == null) return;

        // 本体スプライト
        if (p1Cursor && p2Cursor)
            spriteRenderer.sprite = bothSelectedSprite;
        else if (p1Cursor)
            spriteRenderer.sprite = p1SelectedSprite;
        else if (p2Cursor)
            spriteRenderer.sprite = p2SelectedSprite;
        else
            spriteRenderer.sprite = normalSprite;

        // -------- 1P --------
        if (p1Cursor && p1sprite != null)
        {
            p1sprite.sprite = headerSprite;
            if (!p1HeaderShown)
            {
                UpdateOriginalPositions(); // ★ ここが重要
                if (p1SlideCoroutine != null) StopCoroutine(p1SlideCoroutine);
                p1SlideCoroutine = StartCoroutine(
                    SlideIn(p1Object.transform, p1OriginalPos, true)
                );
                UpdateStatDisplays(true);
                p1HeaderShown = true;
            }
        }
        else
        {
            p1HeaderShown = false;
        }

        // -------- 2P --------
        if (p2Cursor && p2sprite != null)
        {
            p2sprite.sprite = headerSprite;
            if (!p2HeaderShown)
            {
                UpdateOriginalPositions(); // ★ ここが重要
                if (p2SlideCoroutine != null) StopCoroutine(p2SlideCoroutine);
                p2SlideCoroutine = StartCoroutine(
                    SlideIn(p2Object.transform, p2OriginalPos, false)
                );
                UpdateStatDisplays(false);
                p2HeaderShown = true;
            }
        }
        else
        {
            p2HeaderShown = false;
        }
    }

    private IEnumerator SlideIn(Transform target, Vector3 originalPos, bool fromLeft)
    {
        Vector3 startPos =
            originalPos + (fromLeft ? Vector3.left : Vector3.right) * slideDistance;

        target.position = startPos;

        float elapsed = 0f;
        while (elapsed < slideDuration)
        {
            target.position = Vector3.Lerp(startPos, originalPos, elapsed / slideDuration);
            elapsed += Time.deltaTime;
            yield return null;
        }

        // target.position = originalPos;
        target.position = new Vector3(target.parent.position.x, target.parent.position.y, target.position.z);
    }

    private void UpdateStatDisplays(bool isP1)
    {
        // 1Pと2Pで、ターゲットとなるSpawnerを切り替える
        StatBarSpawner speedTarget = isP1 ? p1SpeedSpawner : p2SpeedSpawner;
        StatBarSpawner physicalTarget = isP1 ? p1PhysicalSpawner : p2PhysicalSpawner;
        StatBarSpawner specialTarget = isP1 ? p1SpecialSpawner : p2SpecialSpawner;

        // 設定された数値（int）をそれぞれのSpawnerに渡す
        if (speedTarget != null) speedTarget.SetStatValue(speedCount);
        if (physicalTarget != null) physicalTarget.SetStatValue(physicalCount);
        if (specialTarget != null) specialTarget.SetStatValue(specialCount);
    }
}
