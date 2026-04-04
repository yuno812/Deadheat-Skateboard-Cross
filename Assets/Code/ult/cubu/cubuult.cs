using UnityEngine;
using System.Collections;

public class cubuult : UltimateAbility
{
    [Header("Settings")]
    [SerializeField] private GameObject orbPrefab;
    [SerializeField] private int orbCount = 3;
    [SerializeField] private float interval = 3f; // 生成間隔
    [SerializeField] private GameObject human;

    private int activeOrbCount = 0; // 現在活動中の玉の数
    private bool isSpawning = false; // 生成中かどうかのフラグ
    Vector3 spawnPos;

    public override void Execute(MovePlayer owner)
    {
        spawnPos = owner.transform.position;
        spawnPos.z = owner.transform.position.z + 0.1f;
        StartCoroutine(SpawnRoutine(owner));
    }

    private IEnumerator SpawnRoutine(MovePlayer owner)
    {
        GameObject orbObj = Instantiate(human, spawnPos, Quaternion.identity);
        isSpawning = true;

        for (int i = 0; i < orbCount; i++)
        {
            SpawnOrb(owner);

            // 最後の弾以外は、次の生成まで interval 秒待機
            if (i < orbCount - 1)
            {
                yield return new WaitForSeconds(interval);
            }
        }

        isSpawning = false;
        Destroy(orbObj);
    }

    private void SpawnOrb(MovePlayer owner)
    {

        GameObject orbObj = Instantiate(orbPrefab, spawnPos, Quaternion.identity);

        cubuHoming orbScript = orbObj.GetComponent<cubuHoming>();
        if (orbScript != null)
        {
            activeOrbCount++;
            // 自分自身(this)を渡して、消滅時に通知をもらうようにする
            orbScript.Initialize(owner, this);
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
        return isSpawning || activeOrbCount > 0;
    }
}
