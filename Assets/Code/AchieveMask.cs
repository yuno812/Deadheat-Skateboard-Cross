using UnityEngine;

public class AchieveMask : MonoBehaviour
{
    [Header("実績設定")]
    [SerializeField] private string achievementId;
    [SerializeField] private GameObject lockOverlayObject;
    // Start is called once before the first execution of Update after the MonoBehaviour is created
    void Start()
    {
        if(AchievementManager.Instance.IsUnlocked(achievementId))
        {
            lockOverlayObject.SetActive(false);
        }
    }
}
