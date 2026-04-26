using UnityEngine;
using System.Collections;
using System.Collections.Generic;

public class BGMManager : MonoBehaviour
{
    [Header("Components")]
    [SerializeField] private AudioSource audioSource;

    [Header("Music Database")]
    [SerializeField] private List<MusicData> allMusic; // 全ての曲リストをインスペクターでセットしてください

    private static BGMManager instance;
    public static BGMManager Instance => instance;

    private Coroutine currentRoutine;

    void Awake()
    {
        if (instance == null)
        {
            instance = this;
            DontDestroyOnLoad(gameObject);
        }
        else
        {
            Destroy(gameObject);
            return;
        }
    }

    void Start() 
    {
        // 保存された曲名を取得
        string savedBGM = PlayerPrefs.GetString("SelectedBGM", "");
        MusicData targetMusic = null;

        if (!string.IsNullOrEmpty(savedBGM)) 
        {
            // 1. 保存された曲名と一致するデータをリストから探す
            targetMusic = allMusic.Find(m => m.name == savedBGM);
        }

        if (targetMusic == null) 
        {
            // 2. 保存がない、または見つからない場合は default フラグが立っている曲を探す
            targetMusic = allMusic.Find(m => m.isDefaultBGM);
        }

        // 対象が見つかった場合は再生を開始
        if (targetMusic != null)
        {
            ChangeBGM(targetMusic);
        }
    }

    // 外部（UIなど）から曲を切り替えるメソッド
    public void ChangeBGM(MusicData data)
    {
        if (data == null || audioSource == null) return;

        // すでに再生中のシーケンスがあれば止める
        if (currentRoutine != null)
        {
            StopCoroutine(currentRoutine);
        }

        // 曲固有の音量を反映
        audioSource.volume = data.volume;

        // 新しい再生処理を開始
        currentRoutine = StartCoroutine(PlayBGMSequence(data.introClip, data.loopClip));
        
        // 設定を保存
        PlayerPrefs.SetString("SelectedBGM", data.name);
    }

    private IEnumerator PlayBGMSequence(AudioClip intro, AudioClip loop)
    {
        audioSource.Stop();
        audioSource.clip = null;

        // イントロの処理
        if (intro != null)
        {
            audioSource.clip = intro;
            audioSource.loop = false;
            audioSource.Play();
            yield return new WaitForSeconds(intro.length);
        }

        // ループの処理
        if (loop != null)
        {
            audioSource.clip = loop;
            audioSource.loop = true;
            audioSource.Play();
        }
        else
        {
            audioSource.Stop();
        }

        currentRoutine = null;
    }
}