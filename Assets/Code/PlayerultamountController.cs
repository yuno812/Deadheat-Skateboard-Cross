using UnityEngine;

public class PlayerultamountController : MonoBehaviour
{
    [Header("Search Settings")]
    [SerializeField] private string playerTag = "Player";
    public int targetPlayerNumber = 1;

    [Header("UI Elements (Slanted Gauge)")]
    public RectTransform fillTransform; // マスクの中にある「中身の画像」
    public float emptyPosX = -300f;    // ゲージが0の時のX座標
    public float fullPosX = 0f;       // ゲージが満タンの時のX座標

    private MovePlayer player;

    void Start()
    {
        // プレイヤー検索（ここは変更なし）
        GameObject[] players = GameObject.FindGameObjectsWithTag(playerTag);
        foreach (var obj in players)
        {
            MovePlayer mp = obj.GetComponent<MovePlayer>();
            if (mp != null && mp.playerNumber == targetPlayerNumber)
            {
                player = mp;
                break;
            }
        }
    }

    void Update()
    {
        if (player == null || fillTransform == null) return;

        // ゲージの割合 (0.0 ～ 1.0) を計算
        float ratio = Mathf.Clamp01(player.ultGauge / player.ultAmount);

        // 割合に応じて、X座標を計算
        // 例：0%ならemptyPosX(-300)、100%ならfullPosX(0)
        float currentX = Mathf.Lerp(emptyPosX, fullPosX, ratio);

        // 座標を反映
        fillTransform.anchoredPosition = new Vector2(currentX, fillTransform.anchoredPosition.y);
    }
}