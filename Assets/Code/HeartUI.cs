using UnityEngine;
using UnityEngine.UI;
using System.Collections.Generic;

public class HeartUI : MonoBehaviour
{
    [SerializeField] private string playerTag = "Player";
    public int targetPlayerNumber = 1;

    public GameObject heartPrefab; // ← PrefabはGameObjectに変更

    private MovePlayer targetPlayer;
    private List<GameObject> heartImages = new List<GameObject>();

    void Start()
    {
        // プレイヤー検索
        GameObject[] players = GameObject.FindGameObjectsWithTag(playerTag);
        foreach (var obj in players)
        {
            MovePlayer mp = obj.GetComponent<MovePlayer>();
            if (mp != null && mp.playerNumber == targetPlayerNumber)
            {
                targetPlayer = mp;
                break;
            }
        }

        if (targetPlayer == null)
        {
            Debug.LogError($"{targetPlayerNumber}P の MovePlayer が見つかりません");
            return;
        }

        //　体力を一旦全削除
        for (int i = 0; i < transform.childCount; i++) {
            Destroy(transform.GetChild(i).gameObject);
        }

        // 最大HPぶんハート生成
        int maxHP = Mathf.CeilToInt(targetPlayer.HP);
        for (int i = 0; i < maxHP; i++)
        {
            GameObject obj = Instantiate(heartPrefab, transform);
            heartImages.Add(obj);
        }
    }

    void Update()
    {
        if (targetPlayer != null)
            UpdateHearts(targetPlayer.HP);
    }

    void UpdateHearts(float hp)
    {
        for (int i = 0; i < heartImages.Count; i++)
        {
            float value = hp - i;
            HeartControll hc = heartImages[i].GetComponent<HeartControll>();
            if (value <= 0)
                hc.SetNull();
            else if (value <= 0.5f)
                hc.SetHalf();
            else
                hc.SetFull();
        }
    }
}
