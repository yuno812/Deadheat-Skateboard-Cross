
using UnityEngine;

public class memeult : UltimateAbility
{
    [Header("Settings")]
    [SerializeField] private GameObject orbPrefab;
    [SerializeField] private int orbCount = 3;
    [SerializeField] private float spawnDistance = 2.0f;

    [Header("Spawn Angle")]
    [SerializeField] private float spawnAngleRange = 80f;

    private int activeOrbCount = 0;

    public override void Execute(MovePlayer owner)
    {
        if (owner == null || orbPrefab == null) return;

        MovePlayer enemy = null;
        MovePlayer[] players =
            FindObjectsByType<MovePlayer>(FindObjectsSortMode.None);

        foreach (var p in players)
        {
            if (p != owner)
            {
                enemy = p;
                break;
            }
        }

        for (int i = 0; i < orbCount; i++)
        {
            SpawnOrb(owner, enemy);
        }
    }

    private void SpawnOrb(MovePlayer owner, MovePlayer enemy)
    {
        Vector2 baseDirection;

        if (enemy != null)
        {
            // 自分から敵へ向かう方向
            Vector2 toEnemy =
                ((Vector2)enemy.transform.position -
                 (Vector2)owner.transform.position).normalized;

            // 敵と反対側を基準にする
            baseDirection = -toEnemy;
        }
        else
        {
            // 敵が見つからない場合はキャラクターの左方向
            baseDirection = Vector2.left;
        }

        // 基準方向から±40度の範囲でランダムに回転
        float randomAngle =
            Random.Range(-spawnAngleRange / 2f, spawnAngleRange / 2f);

        Vector2 spawnDirection =
            Quaternion.Euler(0f, 0f, randomAngle) * baseDirection;

        Vector3 spawnOffset =
            (Vector3)(spawnDirection * spawnDistance);

        Vector3 spawnPos = owner.transform.position + spawnOffset;
        spawnPos.z = -2f;

        GameObject orbObj =
            Instantiate(orbPrefab, spawnPos, Quaternion.identity);

        orbObj.transform.up = spawnDirection;

        Homing orbScript = orbObj.GetComponent<Homing>();

        if (orbScript != null)
        {
            activeOrbCount++;
            orbScript.Initialize(
                owner,
                enemy != null ? enemy.transform : null,
                this
            );
        }
    }

    public void NotifyOrbDestroyed()
    {
        activeOrbCount = Mathf.Max(0, activeOrbCount - 1);
    }

    public override bool IsActive()
    {
        return activeOrbCount > 0;
    }
}
