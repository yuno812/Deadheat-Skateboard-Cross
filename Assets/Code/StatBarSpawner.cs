using UnityEngine;
using System.Collections.Generic;

public class StatBarSpawner : MonoBehaviour
{
    [Header("Settings")]
    public GameObject barElementPrefab; // バー1個分のSpriteプレハブ
    public float startX = 2.1f;         // 最初の生成位置（相対座標）
    public float spacing = 0.4f;        // 配置する間隔

    private List<GameObject> barElements = new List<GameObject>();

    // 外部から「何個表示するか」を渡して呼び出す
    public void SetStatValue(int count)
    {
        // 1. 既存のプレハブをすべて削除
        foreach (var obj in barElements)
        {
            if (obj != null) Destroy(obj);
        }
        barElements.Clear();

        // 2. 指定された数だけプレハブを生成して並べる
        for (int i = 0; i < count; i++)
        {
            // 親(this)の子として生成
            GameObject obj = Instantiate(barElementPrefab, transform);
            
            // 相対座標で計算（startXから始まり、spacingずつズレる）
            Vector3 localPos = new Vector3(startX + (i * spacing), 0f, 0f);
            
            obj.transform.localPosition = localPos;
            obj.transform.localRotation = Quaternion.identity;
            
            barElements.Add(obj);
        }
    }
}