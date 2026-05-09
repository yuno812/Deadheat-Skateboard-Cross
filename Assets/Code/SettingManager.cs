using UnityEngine;
using UnityEngine.UI;

/// <summary>
/// 音量と明るさを管理するマネージャー。
/// シーンをまたいでも設定を維持し、PlayerPrefsで保存します。
/// </summary>
public class SettingsManager : MonoBehaviour
{
    public static SettingsManager Instance;

    [Header("Audio Settings")]
    [SerializeField] private Slider volumeSlider;

    [Header("Brightness Settings")]
    [SerializeField] private Slider brightnessSlider;
    [SerializeField] private Image brightnessOverlay; // 画面全体を覆う黒いImage

    private const string VolumeKey = "SavedVolume";
    private const string BrightnessKey = "SavedBrightness";

    void Awake()
    {
        // シングルトンパターン
        if (Instance == null)
        {
            Instance = this;
            // このオブジェクト（および子要素の画像など）をシーン移動で破棄しないようにする
            DontDestroyOnLoad(gameObject);
            
            // 初回起動時の初期化
            LoadAndApplySettings();
        }
        else
        {
            // 二回目以降、設定シーンに戻ってきた時、
            // スライダー（UI）の参照だけを引き渡し、Overlay（実体）の上書きは慎重に行います。
            Instance.UpdateReferences(volumeSlider, brightnessSlider, brightnessOverlay);
            
            // 自分自身（新しく作られた重複分）は破棄
            Destroy(gameObject);
        }
    }

    void Start()
    {
        // 最初のシーンでのみ実行される初期化
        if (Instance == this)
        {
            RefreshUI();
        }
    }

    private void LoadAndApplySettings()
    {
        float savedVolume = PlayerPrefs.GetFloat(VolumeKey, 0.5f);
        float savedBrightness = PlayerPrefs.GetFloat(BrightnessKey, 1.0f);

        SetVolume(savedVolume);
        SetBrightness(savedBrightness);
    }

    private void RefreshUI()
    {
        float savedVolume = PlayerPrefs.GetFloat(VolumeKey, 0.5f);
        float savedBrightness = PlayerPrefs.GetFloat(BrightnessKey, 1.0f);
        SetupUI(volumeSlider, brightnessSlider, savedVolume, savedBrightness);
    }

    private void SetupUI(Slider vol, Slider bright, float vVal, float bVal)
    {
        if (vol != null)
        {
            vol.onValueChanged.RemoveAllListeners();
            vol.value = vVal;
            vol.onValueChanged.AddListener(SetVolume);
        }

        if (bright != null)
        {
            bright.onValueChanged.RemoveAllListeners();
            bright.value = bVal;
            bright.onValueChanged.AddListener(SetBrightness);
        }
    }

    public void UpdateReferences(Slider vol, Slider bright, Image overlay)
    {
        Debug.Log("SettingsManager: 参照を更新します");

        // --- 修正ポイント ---
        // もし既に有効な Overlay（子オブジェクトなど）を持っているなら、
        // 消えてしまう運命の「新しいシーンのOverlay」で上書きしないようにします。
        if (brightnessOverlay == null && overlay != null)
        {
            brightnessOverlay = overlay;
        }

        // スライダー（UI）はシーンごとに新しくなるので必ず更新する
        volumeSlider = vol;
        brightnessSlider = bright;

        // 現在の保存値をUIに反映
        RefreshUI();
        
        // 保存されている明るさを現在の（生き残っている）Overlayに再適用
        SetBrightness(PlayerPrefs.GetFloat(BrightnessKey, 1.0f)); 
    }

    public void SetVolume(float volume)
    {
        AudioListener.volume = volume;
        PlayerPrefs.SetFloat(VolumeKey, volume);
        Debug.Log($"SettingsManager: 音量を {volume} に変更しました");
    }

    public void SetBrightness(float value)
    {
        PlayerPrefs.SetFloat(BrightnessKey, value);

        if (brightnessOverlay != null)
        {
            float alpha = 1.0f - value;
            Color color = brightnessOverlay.color;
            color.a = alpha;
            brightnessOverlay.color = color;
        }
        else
        {
            // もし何らかの理由で参照が外れていたら、子要素から探し出す（保険）
            brightnessOverlay = GetComponentInChildren<Image>();
            
            // それでもなければ警告
            if (brightnessOverlay == null)
            {
                Debug.LogWarning("SettingsManager: BrightnessOverlay が見つかりません。");
            }
        }
    }
}