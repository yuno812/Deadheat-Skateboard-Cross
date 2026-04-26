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
            DontDestroyOnLoad(gameObject);
            
            // 初回起動時の初期化
            LoadAndApplySettings();
        }
        else
        {
            // 二回目以降、設定シーンに戻ってきた時、
            // 新しく配置されたインスペクターの参照を、生き残っている Instance に引き渡す
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
            float savedVolume = PlayerPrefs.GetFloat(VolumeKey, 0.5f);
            float savedBrightness = PlayerPrefs.GetFloat(BrightnessKey, 1.0f);

            // リスナーの登録（重複登録を避けるため一度クリアしてから登録）
            SetupUI(volumeSlider, brightnessSlider, savedVolume, savedBrightness);
        }
    }

    // 保存された値を読み込んで反映する
    private void LoadAndApplySettings()
    {
        float savedVolume = PlayerPrefs.GetFloat(VolumeKey, 0.5f);
        float savedBrightness = PlayerPrefs.GetFloat(BrightnessKey, 1.0f);

        SetVolume(savedVolume);
        SetBrightness(savedBrightness);
    }

    // UIの初期値設定とリスナー登録
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

    // 参照を更新し、イベントを再登録する
    public void UpdateReferences(Slider vol, Slider bright, Image overlay)
    {
        Debug.Log("SettingsManager: 参照を更新します");

        // 新しい参照をセット
        volumeSlider = vol;
        brightnessSlider = bright;
        brightnessOverlay = overlay;

        // 現在の保存値を適用しつつUIを再構築
        float currentVol = PlayerPrefs.GetFloat(VolumeKey, 0.5f);
        float currentBright = PlayerPrefs.GetFloat(BrightnessKey, 1.0f);

        SetupUI(volumeSlider, brightnessSlider, currentVol, currentBright);
        
        // 新しいシーンのオーバーレイにも即座に現在の明るさを反映
        SetBrightness(currentBright); 
    }

    public void SetVolume(float volume)
    {
        AudioListener.volume = volume;
        PlayerPrefs.SetFloat(VolumeKey, volume);
        // デバッグ用ログ
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
            // デバッグ用ログ
            Debug.Log($"SettingsManager: 明るさを {value} (Alpha: {alpha}) に変更しました");
        }
        else
        {
            Debug.LogWarning("SettingsManager: BrightnessOverlay が割り当てられていません！");
        }
    }
}