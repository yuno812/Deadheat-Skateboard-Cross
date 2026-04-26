using UnityEngine;
using UnityEngine.UI;
using UnityEngine.EventSystems; // イベントシステムを使用するために追加
using System.Collections.Generic;
using TMPro;

public class BGMSelectorUI : MonoBehaviour
{
    [Header("Data")]
    [SerializeField] private List<MusicData> musicList; 

    [Header("UI References")]
    [SerializeField] private ScrollRect scrollRect;     
    [SerializeField] private Transform contentTransform; 
    [SerializeField] private GameObject musicItemPrefab; 

    void Start()
    {
        PopulateList();
    }

    private void PopulateList()
    {
        if (contentTransform == null) return;

        foreach (Transform child in contentTransform)
        {
            Destroy(child.gameObject);
        }

        foreach (var music in musicList)
        {
            if (music == null) continue;

            GameObject item = Instantiate(musicItemPrefab, contentTransform);
            
            // Z座標の初期化
            item.transform.localPosition = new Vector3(item.transform.localPosition.x, item.transform.localPosition.y, 0);

            // テキストの設定
            var tmpText = item.GetComponentInChildren<TextMeshProUGUI>();
            if (tmpText != null) tmpText.text = music.musicName;
            else
            {
                var legacyText = item.GetComponentInChildren<Text>();
                if (legacyText != null) legacyText.text = music.musicName;
            }

            // ボタンのクリックイベント設定
            Button btn = item.GetComponent<Button>();
            if (btn != null)
            {
                btn.onClick.AddListener(() => OnMusicSelected(music));

                // --- ★修正：ドラッグイベントをボタンで止める処理 ---
                // ボタンに EventTrigger を追加し、Dragイベントを「何もしない」で受け取ることで、
                // 親の ScrollRect にドラッグが伝わるのを防ぎます。
                EventTrigger trigger = item.GetComponent<EventTrigger>();
                if (trigger == null) trigger = item.AddComponent<EventTrigger>();

                EventTrigger.Entry entry = new EventTrigger.Entry();
                entry.eventID = EventTriggerType.Drag;
                entry.callback.AddListener((data) => {
                    // 何もしない（ドラッグイベントをここで消費する）
                });
                trigger.triggers.Add(entry);
            }
        }

        // レイアウトの強制更新
        Canvas.ForceUpdateCanvases();
        
        if (scrollRect != null)
        {
            scrollRect.StopMovement();
            scrollRect.verticalNormalizedPosition = 1f; 
            LayoutRebuilder.ForceRebuildLayoutImmediate(contentTransform.GetComponent<RectTransform>());
            scrollRect.SendMessage("OnRectTransformDimensionsChange", SendMessageOptions.DontRequireReceiver);
        }
    }

    private void OnMusicSelected(MusicData data)
    {
        if (BGMManager.Instance != null)
        {
            BGMManager.Instance.ChangeBGM(data);
            Debug.Log($"曲を切り替えました: {data.musicName}");
        }
    }
}