using UnityEngine;

public class PressAnyButtonMove : MonoBehaviour
{
    [Header("移動の設定")]
    public float amplitude = 20f; // 上下に動く幅
    public float speed = 2f;     // 動くスピード

    private Vector3 startPos;

    void Start()
    {
        // 最初の位置を覚えておく
        startPos = transform.localPosition;
    }

    void Update()
    {
        // 時間（Time.time）にスピードをかけてSin波を計算
        // Sinは -1.0 ～ 1.0 の間を往復する
        float yOffset = Mathf.Sin(Time.time * speed) * amplitude;

        // 最初の位置に計算したズレ（yOffset）を足す
        transform.localPosition = startPos + new Vector3(0, yOffset, 0);
    }
}
