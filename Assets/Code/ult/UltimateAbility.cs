using UnityEngine;

public abstract class UltimateAbility : MonoBehaviour
{
    public abstract void Execute(MovePlayer owner);

    // ウルトが活動中（ゲージを溜めない期間）かどうかを判定する
    // デフォルトは false（溜まり続ける）
    public virtual bool IsActive()
    {
        return false;
    }
}
