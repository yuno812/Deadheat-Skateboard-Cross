using UnityEngine;

public class JumpBoostTrigger : MonoBehaviour
{
    [SerializeField] private float jumpMultiplier = 1.1f;

    private void OnTriggerEnter2D(Collider2D other)
    {
        // ★ GetComponent ではなく GetComponentInParent を使用
        // これで孫(Collider)が触れても、一番上の親(MovePlayer)を見つけ出せます
        MovePlayer player = other.GetComponentInParent<MovePlayer>();

        if (player != null)
        {
            player.SetJumpMultiplier(jumpMultiplier);
            Debug.Log($"親オブジェクトの {player.name} を検知して強化しました！");
        }
    }

    private void OnTriggerExit2D(Collider2D other)
    {
        MovePlayer player = other.GetComponentInParent<MovePlayer>();
        if (player != null)
        {
            player.SetJumpMultiplier(1.0f);
            Debug.Log("エリア外に出たので強化を解除しました");
        }
    }
}