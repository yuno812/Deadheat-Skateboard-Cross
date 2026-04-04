using UnityEngine;

public class memeult : UltimateAbility
{
    [Header("Settings")]
    [SerializeField] private GameObject orbPrefab;
    [SerializeField] private int orbCount = 3;
    [SerializeField] private float spawnDistance = 2.0f;

    private int activeOrbCount = 0; // 現在活動中の玉の数

    public override void Execute(MovePlayer owner)
    {
        MovePlayer enemy = null;
        MovePlayer[] players = FindObjectsByType<MovePlayer>(FindObjectsSortMode.None);
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
        float randomAngle = Random.Range(0f, 360f);
        Vector3 spawnOffset = Quaternion.Euler(0, 0, randomAngle) * Vector3.up * spawnDistance;
        Vector3 spawnPos = owner.transform.position + spawnOffset;
        spawnPos.z = -2f;

        GameObject orbObj = Instantiate(orbPrefab, spawnPos, Quaternion.identity);
        orbObj.transform.up = spawnOffset.normalized;

        Homing orbScript = orbObj.GetComponent<Homing>();
        if (orbScript != null)
        {
            activeOrbCount++;
            // 自分自身(this)を渡して、消滅時に通知をもらうようにする
            orbScript.Initialize(owner, enemy != null ? enemy.transform : null, this);
        }
    }

    // 玉が消える時に玉側から呼ばれる
    public void NotifyOrbDestroyed()
    {
        activeOrbCount--;
    }

    // ★ここで「玉がある間はアクティブ」と定義する
    public override bool IsActive()
    {
        return activeOrbCount > 0;
    }
}
