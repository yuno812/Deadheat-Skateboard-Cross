using UnityEngine;

public class cubuHoming : MonoBehaviour
{
    [Header("Movement Settings")]
    public float speed = 8f;            // 弾速
    public float rotationSpeed = 300f;  // 旋回速度

    [Header("Heal Settings")]
    [SerializeField] private float baseHeal = 1f; // 基本回復量

    private Transform target;
    private Rigidbody2D rb;
    private float timer = 0f;
    private bool isHoming = true;
    private bool isDestroyed = false;
    private MovePlayer ownerPlayer;
    private cubuult sourceAbility;

    public void Initialize(MovePlayer owner, cubuult ability)
    {
        ownerPlayer = owner;
        target = owner.transform;
        sourceAbility = ability;
        rb = GetComponent<Rigidbody2D>();
        
        if (rb != null)
        {
            rb.gravityScale = 0;
            rb.linearDamping = 0; // 速度が落ちないように
        }
    }

    void FixedUpdate()
    {
        Vector2 direction = (Vector2)target.position - rb.position;
        direction.Normalize();

        float rotateAmount = Vector3.Cross(direction, transform.up).z;
        rb.angularVelocity = -rotateAmount * rotationSpeed;
        rb.linearVelocity = transform.up * speed;
    }

    private void OnTriggerEnter2D(Collider2D other)
    {
        MovePlayer hitPlayer = other.GetComponentInParent<MovePlayer>();
        Debug.Log("hit");
        if (hitPlayer != null)
        {
            // 自分自身（オーナー）なら無視
            if (hitPlayer == ownerPlayer)
            {
                ownerPlayer.HP += baseHeal;
                SelfDestruct();
            }
        }
        // 地面などは OnTrigger なので無視して通り抜ける設定
    }

    // 消滅処理（ウルト側に通知してから消える）
    private void SelfDestruct()
    {
        if (isDestroyed) return;
        isDestroyed = true;

        if (sourceAbility != null)
        {
            sourceAbility.NotifyOrbDestroyed();
        }
        Destroy(gameObject);
    }
}
