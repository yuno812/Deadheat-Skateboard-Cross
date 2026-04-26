using UnityEngine;

[CreateAssetMenu(fileName = "NewMusicData", menuName = "Settings/MusicData")]
public class MusicData : ScriptableObject
{
    public string musicName;      // 表示する曲名
    
    [Range(0f, 1f)]
    public float volume = 1f;     // この曲固有の音量 (デフォルト 1.0)

    [Header("Startup Setting")]
    public bool isDefaultBGM;     // ゲーム開始時に何も設定されていない場合にデフォルトで再生するか

    [Header("Audio Clips")]
    public AudioClip introClip;   // イントロ
    public AudioClip loopClip;    // ループ
}