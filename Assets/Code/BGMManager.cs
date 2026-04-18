using UnityEngine;
using System.Collections;

public class BGMManager : MonoBehaviour
{
    [Header("Audio Clips")]
    [SerializeField] private AudioClip introClip; // 最初に一度だけ流す曲
    [SerializeField] private AudioClip loopClip;  // その後にループさせる曲

    [Header("Components")]
    [SerializeField] private AudioSource audioSource; // 再生用のAudioSource

    private static BGMManager instance;

    void Awake()
    {
        // シングルトンパターン：既にBGMManagerが存在する場合は、新しく作られた方を破棄する
        if (instance == null)
        {
            instance = this;
            // このオブジェクトをシーン切り替えで破棄しないように設定
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
        // 重複チェックで破棄されなかったインスタンスのみ再生を開始
        if (instance == this && audioSource != null && introClip != null && loopClip != null)
        {
            StartCoroutine(PlayBGMSequence());
        }
    }

    private IEnumerator PlayBGMSequence()
    {
        // 1. イントロを再生（ループなし）
        audioSource.clip = introClip;
        audioSource.loop = false;
        audioSource.Play();

        // 2. イントロが終わるまで待機
        yield return new WaitForSeconds(introClip.length);

        // 3. ループ曲に切り替えて再生（ループあり）
        audioSource.clip = loopClip;
        audioSource.loop = true;
        audioSource.Play();
    }
}