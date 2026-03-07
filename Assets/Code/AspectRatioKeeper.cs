using UnityEngine;
using UnityEngine.SceneManagement;

public class AspectRatioKeeper : MonoBehaviour
{
    [Header("Target Aspect Ratio")]
    [SerializeField] private float targetWidth = 16f;
    [SerializeField] private float targetHeight = 9f;

    private static AspectRatioKeeper instance;
    private Camera targetCamera;

    void Awake()
    {
        // シングルトンパターン：重複を防ぎ、シーンをまたいでも消えないようにする
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
        FindAndApplyCamera();
    }

    void Update()
    {
        // カメラが未設定、またはシーン移動で消失した場合は再取得
        if (targetCamera == null)
        {
            FindAndApplyCamera();
        }

        if (targetCamera != null)
        {
            UpdateAspectRatio();
        }
    }

    private void FindAndApplyCamera()
    {
        // シーン内のメインカメラを探す
        targetCamera = Camera.main;
        if (targetCamera != null)
        {
            UpdateAspectRatio();
        }
    }

    private void UpdateAspectRatio()
    {
        if (targetCamera == null) return;

        // 現在の画面のアスペクト比
        float windowAspect = (float)Screen.width / (float)Screen.height;
        // 理想のアスペクト比
        float targetAspect = targetWidth / targetHeight;
        
        // 理想の比率に対する現在の比率の割合
        float scaleHeight = windowAspect / targetAspect;

        // カメラの描画範囲（Viewport Rect）を計算
        if (scaleHeight < 1.0f)
        {
            // 画面がターゲットより縦長の場合（上下に黒帯）
            Rect rect = targetCamera.rect;
            rect.width = 1.0f;
            rect.height = scaleHeight;
            rect.x = 0;
            rect.y = (1.0f - scaleHeight) / 2.0f;
            targetCamera.rect = rect;
        }
        else
        {
            // 画面がターゲットより横長の場合（左右に黒帯）
            float scaleWidth = 1.0f / scaleHeight;
            Rect rect = targetCamera.rect;
            rect.width = scaleWidth;
            rect.height = 1.0f;
            rect.x = (1.0f - scaleWidth) / 2.0f;
            rect.y = 0;
            targetCamera.rect = rect;
        }
    }
}