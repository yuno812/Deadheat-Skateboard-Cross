using UnityEngine;

public class Bumper : MonoBehaviour
{
    [SerializeField] private float knockbackPower = 25f;

    private void OnCollisionEnter2D(Collision2D collision)
    {
        MovePlayer player = collision.collider.GetComponentInParent<MovePlayer>();

        if (player == null)
            return;

        Rigidbody2D rb = player.GetComponent<Rigidbody2D>();

        // バンパーの中心 → プレイヤー方向
        Vector2 direction =
            ((Vector2)player.transform.position - (Vector2)transform.position).normalized;

        // 攻撃を受けた時と同じように吹き飛ばす
        rb.linearVelocity = direction * knockbackPower;
    }
}