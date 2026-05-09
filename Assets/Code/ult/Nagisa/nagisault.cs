using UnityEngine;
using System.Collections;

/// <summary>
/// ナギサの必殺技：壁などの当たり判定を維持しつつ、物理演算（重力等）を無視して突進する。
/// </summary>
public class NagisaUltimate : UltimateAbility
{
    [Header("Dash Settings")]
    [SerializeField] private float dashSpeed = 40f;    // 突進速度
    [SerializeField] private float dashDuration = 0.3f; // 突進時間
    
    private bool isDashing = false;

    public override void Execute(MovePlayer owner)
    {
        if (isDashing) return;
        StartCoroutine(DashRoutine(owner));
    }

    private IEnumerator DashRoutine(MovePlayer owner)
    {
        isDashing = true;
        
        Rigidbody2D rb = owner.GetComponent<Rigidbody2D>();
        
        // 1. 元の状態を保存
        float originalGravity = rb.gravityScale;
        Vector2 originalVelocity = rb.linearVelocity;

        // 2. 物理演算の影響を最小化
        rb.gravityScale = 0f; // 重力を無視
        
        // 進行方向（プレイヤーの右方向）を決定
        Vector2 dashDirection = owner.transform.right;

        float timer = 0f;
        while (timer < dashDuration)
        {
            // 毎フレーム速度を直接上書きすることで、摩擦や他の力を無視して一定速度で進む
            // Dynamicなので、壁（Collider）があればそこで止まります
            rb.linearVelocity = dashDirection * dashSpeed;
            
            timer += Time.deltaTime;
            yield return null; 
        }

        // 3. 状態を元に戻す
        rb.gravityScale = originalGravity;
        
        // 突進直後に急停止させたい場合は Vector2.zero、余韻を残したい場合は適度な速度を与える
        rb.linearVelocity = dashDirection * (dashSpeed * 0.2f);

        isDashing = false;
    }

    public override bool IsActive()
    {
        return isDashing;
    }
}