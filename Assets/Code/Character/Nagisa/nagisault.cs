
using System.Collections;
using UnityEngine;

public class NagisaUltimate : UltimateAbility
{
    [Header("Dash Settings")]
    [SerializeField] private float dashSpeed = 40f;
    [SerializeField] private float dashDuration = 0.3f;

    private bool isDashing = false;

    public override void Execute(MovePlayer owner)
    {
        if (isDashing || owner == null) return;

        Rigidbody2D rb = owner.GetComponent<Rigidbody2D>();
        if (rb == null) return;

        StartCoroutine(DashRoutine(owner, rb));
    }

    
    private IEnumerator DashRoutine(MovePlayer owner, Rigidbody2D rb)
    {
        isDashing = true;

        if (AchievementManager.Instance != null)
            AchievementManager.Instance.UnlockAchievement("nagisa");

        float originalGravity = rb.gravityScale;
        Vector2 direction = ((Vector2)owner.transform.right).normalized;

        rb.gravityScale = 0f;
        rb.linearVelocity = Vector2.zero;
        rb.angularVelocity = 0f;

        float elapsed = 0f;

        while (elapsed < dashDuration)
        {
            if (owner == null || rb == null)
                break;

            // 物理フレームごとの移動量を一定にする
            float step = dashSpeed * Time.fixedDeltaTime;
            rb.MovePosition(rb.position + direction * step);

            yield return new WaitForFixedUpdate();
            elapsed += Time.fixedDeltaTime;
        }

        if (rb != null)
        {
            rb.gravityScale = originalGravity;
            rb.linearVelocity = direction * (dashSpeed * 0.2f);
            rb.angularVelocity = 0f;
        }

        isDashing = false;
    }

}
