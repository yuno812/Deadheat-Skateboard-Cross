using UnityEngine;

public class RotationFixer : MonoBehaviour
{
    void LateUpdate()
    {
        // 親の親（祖父母）の回転に全軸同期させる
        // 確認（nullチェック）なしで直接適用
        transform.rotation = transform.parent.parent.rotation;
    }
}