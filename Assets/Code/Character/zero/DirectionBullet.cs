
using UnityEngine;

[RequireComponent(typeof(Rigidbody2D), typeof(Collider2D))]
public class DirectionBullet : MonoBehaviour
{
    private Rigidbody2D rb;
    private MovePlayer ownerPlayer;

    private float speed;
    private float damage;
    private bool hasDamaged = false;

    private Camera targetCamera;

    public void Initialize(
        MovePlayer owner,
        Vector2 direction,
        float bulletSpeed,
        float bulletDamage)
    {
        ownerPlayer = owner;
        speed = bulletSpeed;
        damage = bulletDamage;
        targetCamera = Camera.main;

        rb = GetComponent<Rigidbody2D>();

        rb.gravityScale = 0f;
        rb.linearDamping = 0f;
        rb.angularVelocity = 0f;
        rb.linearVelocity = direction.normalized * speed;
    }

    private void FixedUpdate()
    {
        if (rb == null) return;

        // 速度を維持して直進
        rb.linearVelocity = (Vector2)transform.up * speed;
    }

    private void OnTriggerEnter2D(Collider2D other)
    {
        MovePlayer hitPlayer = other.GetComponentInParent<MovePlayer>();

        // プレイヤー以外に当たった場合は無視
        if (hitPlayer == null) return;

        // 発射した本人には当たらない
        if (hitPlayer == ownerPlayer) return;

        // 1発につきダメージ判定は1回だけ
        if (hasDamaged) return;

        hasDamaged = true;
        hitPlayer.HP -= damage;

        Debug.Log($"{hitPlayer.name} に命中！ ダメージ: {damage}");
    }

    private void Update()
    {
        // カメラの表示範囲外に出たら消滅
        if (targetCamera == null) return;

        Vector3 viewportPos =
            targetCamera.WorldToViewportPoint(transform.position);

        if (viewportPos.x < 0f || viewportPos.x > 1f ||
            viewportPos.y < 0f || viewportPos.y > 1f ||
            viewportPos.z < 0f)
        {
            Destroy(gameObject);
        }
    }
}
