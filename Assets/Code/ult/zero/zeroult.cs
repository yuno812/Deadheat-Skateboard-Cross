using System.Collections;
using UnityEngine;

public class zeroult : UltimateAbility
{
    [Header("弾の設定")]
    [SerializeField] private GameObject bulletPrefab;
    [SerializeField] private float bulletSpeed = 12f;
    [SerializeField] private float bulletDamage = 2f;

    [Header("発射設定")]
    [SerializeField] private float stopDuration = 0.5f;
    [SerializeField] private float spawnDistance = 1f;

    private bool isActive = false;

    public override void Execute(MovePlayer owner)
    {
        if (isActive || owner == null || bulletPrefab == null)
            return;

        StartCoroutine(FireRoutine(owner));
    }

    private IEnumerator FireRoutine(MovePlayer owner)
    {
        isActive = true;

        Rigidbody2D ownerRb = owner.GetComponent<Rigidbody2D>();

        if (ownerRb == null)
        {
            isActive = false;
            yield break;
        }

        // ウルト用スプライトに切り替える
        owner.ultimatebool = true;

        // 元の設定を保存
        RigidbodyConstraints2D originalConstraints = ownerRb.constraints;
        float originalGravityScale = ownerRb.gravityScale;

        // 発動時に慣性をリセット
        ownerRb.linearVelocity = Vector2.zero;
        ownerRb.angularVelocity = 0f;

        // 停止中は重力を無効化し、位置と回転を固定
        ownerRb.gravityScale = 0f;
        ownerRb.constraints |= RigidbodyConstraints2D.FreezePosition;
        ownerRb.constraints |= RigidbodyConstraints2D.FreezeRotation;

        float elapsed = 0f;

        while (elapsed < stopDuration)
        {
            if (owner == null || ownerRb == null)
            {
                isActive = false;
                yield break;
            }

            ownerRb.linearVelocity = Vector2.zero;
            ownerRb.angularVelocity = 0f;

            elapsed += Time.fixedDeltaTime;
            yield return new WaitForFixedUpdate();
        }

        if (owner == null || ownerRb == null)
        {
            isActive = false;
            yield break;
        }

        // 停止解除
        ownerRb.constraints = originalConstraints;
        ownerRb.gravityScale = originalGravityScale;
        ownerRb.linearVelocity = Vector2.zero;
        ownerRb.angularVelocity = 0f;

        // ウルト用スプライトを解除
        owner.ultimatebool = false;

        // キャラクターの右方向へ発射
        Vector2 direction = owner.transform.right.normalized;

        Vector3 spawnPosition =
            owner.transform.position + (Vector3)(direction * spawnDistance);

        GameObject bullet = Instantiate(
            bulletPrefab,
            spawnPosition,
            Quaternion.FromToRotation(Vector3.up, direction)
        );

        DirectionBullet bulletScript =
            bullet.GetComponent<DirectionBullet>();

        if (bulletScript != null)
        {
            bulletScript.Initialize(
                owner,
                direction,
                bulletSpeed,
                bulletDamage
            );
        }

        isActive = false;
    }

    public override bool IsActive()
    {
        return isActive;
    }
}