using UnityEngine;
using System.Collections;

public class maidenult : UltimateAbility
{
    [Header("エリアPrefab")]
    [SerializeField] private GameObject moveDebuffPrefab;
    [SerializeField] private GameObject jumpDebuffPrefab;

    [Header("エリア設定")]
    [SerializeField] private float duration = 4f;

    [Header("カメラ")]
    [SerializeField] private Camera targetCamera;

    private bool isActive = false;

    public override void Execute(MovePlayer owner)
    {
        if (isActive) return;

        StartCoroutine(CreateDebuffArea(owner));
    }

    public override bool IsActive()
    {
        return isActive;
    }

    private IEnumerator CreateDebuffArea(MovePlayer owner)
    {
        isActive = true;

        if (targetCamera == null)
        {
            targetCamera = Camera.main;
        }

        if (targetCamera == null)
        {
            isActive = false;
            yield break;
        }

        // 発動者が画面の左右どちらにいるか判定
        Vector3 screenPos =
            targetCamera.WorldToViewportPoint(owner.transform.position);

        bool ownerIsLeft = screenPos.x < 0.5f;

        // 左側の発動者なら右側にジャンプ低下、
        // 右側の発動者なら左側に移動低下
        GameObject prefab =
            ownerIsLeft ? jumpDebuffPrefab : moveDebuffPrefab;

        if (prefab == null)
        {
            isActive = false;
            yield break;
        }

        // カメラに映る範囲のワールドサイズ
        float height = targetCamera.orthographicSize * 2f;
        float width = height * targetCamera.aspect;

        // 発動者と反対側の画面半分の中心
        float centerX = targetCamera.transform.position.x
            + (ownerIsLeft ? width / 4f : -width / 4f);

        float centerY = targetCamera.transform.position.y;

        GameObject area = Instantiate(
            prefab,
            new Vector3(centerX, centerY, 0f),
            Quaternion.identity
        );

        // エリアの生成者を登録
        DebuffArea debuffArea = area.GetComponent<DebuffArea>();

        if (debuffArea != null)
        {
            debuffArea.SetOwner(owner);
        }

        // エリアを画面半分の大きさにする
        area.transform.localScale =
            new Vector3(width / 2f, height, 1f);

        // 効果時間中はゲージを回復させない
        yield return new WaitForSeconds(duration);

        if (area != null)
        {
            Destroy(area);
        }

        isActive = false;
    }
}