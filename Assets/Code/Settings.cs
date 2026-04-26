using UnityEngine;

public class Settings : MonoBehaviour
{
    private RectTransform rectTransform;

    [Header("Positions (Anchored Position)")]
    public Vector2 pointA;
    public Vector2 pointB;
    
    [Header("Movement Settings")]
    public float moveTime = 0.5f; // A→Bにかかる時間
    public bool set; // trueでpointBへ、falseでpointAへ

    private float moveSpeed;

    void Awake()
    {
        // UI操作にはRectTransformが必要
        rectTransform = GetComponent<RectTransform>();
    }

    void Start()
    {
        // 初期位置をpointAに設定
        rectTransform.anchoredPosition = pointA;

        // A→B の距離を moveTime 秒で移動するための速度を計算
        moveSpeed = Vector3.Distance(pointA, pointB) / moveTime;
    }

    void Update()
    {
        // ターゲットを決定
        Vector2 target = set ? pointB : pointA;

        // MoveTowardsでスムーズに移動
        rectTransform.anchoredPosition = Vector2.MoveTowards(
            rectTransform.anchoredPosition,
            target,
            moveSpeed * Time.deltaTime
        );
    }

    public void True()
    {
        set = true;
    }

    public void False()
    {
        set = false;
    }
}