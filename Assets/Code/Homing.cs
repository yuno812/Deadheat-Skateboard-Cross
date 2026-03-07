using UnityEngine;

public class Homing : MonoBehaviour
{
    [Header("Movement Settings")]
    public float speed = 8f;            // 弾速
    public float rotationSpeed = 300f;  // 旋回速度
    public float homingDuration = 5f;   // 追尾時間（秒）
    public float totalLifeTime = 7f;    // 消滅するまでの全時間

    [Header("Damage Settings")]
    [SerializeField] private float baseDamage = 1f; // 基本ダメージ量

    private Transform target;
    private Rigidbody2D rb;
    private float timer = 0f;
    private bool isHoming = true;
    private bool isDestroyed = false;
    private MovePlayer ownerPlayer;
    private memeult sourceAbility;

    public void Initialize(MovePlayer owner, Transform enemyTarget, memeult ability)
    {
        ownerPlayer = owner;
        target = enemyTarget;
        sourceAbility = ability;
        rb = GetComponent<Rigidbody2D>();
        
        if (rb != null)
        {
            rb.gravityScale = 0;
            rb.linearDamping = 0; // 速度が落ちないように
        }
        
        Invoke(nameof(SelfDestruct), totalLifeTime);
    }

    void FixedUpdate()
    {
        if (rb == null) return;
        timer += Time.fixedDeltaTime;

        // 5秒間は追尾、その後は直進
        if (isHoming && timer < homingDuration && target != null)
        {
            Vector2 direction = (Vector2)target.position - rb.position;
            direction.Normalize();

            float rotateAmount = Vector3.Cross(direction, transform.up).z;
            rb.angularVelocity = -rotateAmount * rotationSpeed;
        }
        else if (isHoming)
        {
            // 追尾モード終了
            isHoming = false;
            rb.angularVelocity = 0;
        }
        
        // 常に正面に進む
        rb.linearVelocity = transform.up * speed;
    }

    private void OnTriggerEnter2D(Collider2D other)
    {
        // 親の MovePlayer を取得
        MovePlayer hitPlayer = other.GetComponentInParent<MovePlayer>();

        if (hitPlayer != null)
        {
            // 自分自身（オーナー）なら無視
            if (hitPlayer == ownerPlayer) return;

            // --- タグによるダメージ倍率判定 ---
            float damageMultiplier = 1.0f;

            // 通常攻撃と同様にタグで判定
            if (other.CompareTag("MainCollision"))
            {
                damageMultiplier = 1.0f; // 弱点は2倍
            }
            else if (other.CompareTag("AttackCollision") || other.CompareTag("TireCollision") || other.CompareTag("Bullet"))
            {
                damageMultiplier = 0.0f; // 装甲は半分
            }

            // 最終ダメージ
            float finalDamage = baseDamage * damageMultiplier;
            hitPlayer.HP -= finalDamage;

            Debug.Log($"{hitPlayer.name} の {other.tag} に命中！ ダメージ: {finalDamage}");

            // --- ノックバック（通常の攻撃を再現） ---
            Rigidbody2D targetRb = hitPlayer.GetComponent<Rigidbody2D>();
            if (targetRb != null)
            {
                Vector2 dir = (hitPlayer.transform.position - transform.position).normalized;
                targetRb.linearVelocity = dir * 25f; // 25fは通常攻撃の勢い
            }

            SelfDestruct();
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
