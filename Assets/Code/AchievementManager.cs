using UnityEngine;
using System;
using System.Collections.Generic;

#if USE_STEAMWORKS
using Steamworks;
#endif

public class AchievementManager : MonoBehaviour
{
    public static AchievementManager Instance { get; private set;}

    [System.Serializable]
    public class Achievement
    {
        public string id;
        public string title;
        [TextArea] public string description;

        [Header("Progression")]
        public int targetProgress = 0;
        [HideInInspector] public int currentProgress;
        [HideInInspector] public bool isUnlocked;
    }

    [Header("Achievement Database")]
    [SerializeField] private List<Achievement> achievements = new List<Achievement>();

    // 実績解除通知用イベント
    public static event Action<Achievement> OnAchievementUnlocked;

    private void Awake()
    {
        if (Instance == null)
        {
            Instance = this;
            DontDestroyOnLoad(gameObject);
            LoadAchievements();
        }
        else
        {
            Destroy(gameObject);
        }
    }
    
    // 実績解除
    public void UnlockAchievement(string id)
    {
        Achievement target = achievements.Find(a => a.id == id);
        if (target == null)
        {
            Debug.LogWarning($"[AchievementManager] 実績ID '{id}' がデータベースに見つかりません。");
            return;
        }

        if (target.isUnlocked) return;

        target.isUnlocked = true;
        target.currentProgress = target.targetProgress;
        SaveAchievementState(target);

        Debug.Log($"[AchievementManager] 実績解除: {target.title} ({target.description})");

        OnAchievementUnlocked?.Invoke(target);

#if USE_STEAMWORKS
        if (SteamManager.Initialized)
        {
            SteamUserStats.SetAchievement(id);
            SteamUserStats.StoreStats();
        }
#endif
    }

    public void AddProgress(string id, int amount)
    {
        Achievement target = achievements.Find(a => a.id == id);
        if (target == null || target.isUnlocked) return;

        target.currentProgress = Mathf.Min(target.currentProgress + amount, target.targetProgress);
        SaveAchievementState(target);

        Debug.Log($"[AchievementManager] 実績進捗 '{target.title}': {target.currentProgress} / {target.targetProgress}");

        if (target.currentProgress >= target.targetProgress && target.targetProgress > 0)
        {
            UnlockAchievement(id);
        }
    }

    // 実績確認
    public bool IsUnlocked(string id)
    {
        Achievement target = achievements.Find(a => a.id == id);
        return target != null && target.isUnlocked;
    }

    private void LoadAchievements()
    {
        foreach (var ach in achievements)
        {
            ach.isUnlocked = PlayerPrefs.GetInt($"Ach_{ach.id}_Unlocked", 0) == 1;
            ach.currentProgress = PlayerPrefs.GetInt($"Ach_{ach.id}_Progress", 0);
        }
    }

    private void SaveAchievementState(Achievement ach)
    {
        PlayerPrefs.SetInt($"Ach_{ach.id}_Unlocked", ach.isUnlocked ? 1 : 0);
        PlayerPrefs.SetInt($"Ach_{ach.id}_Progress", ach.currentProgress);
        PlayerPrefs.Save();
    }

    // デバッグ用にセーブデータをリセット
    [ContextMenu("Reset Achievements (Debug)")]
    public void ResetAllAchievements()
    {
        foreach (var ach in achievements)
        {
            ach.isUnlocked = false;
            ach.currentProgress = 0;
            PlayerPrefs.DeleteKey($"Ach_{ach.id}_Unlocked");
            PlayerPrefs.DeleteKey($"Ach_{ach.id}_Progress");
        }
        PlayerPrefs.Save();
        Debug.Log("[AchievementManager] すべての実績を初期化しました。");
    }
}