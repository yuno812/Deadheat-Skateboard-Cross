using UnityEngine;

[CreateAssetMenu(fileName = "PlayerState", menuName = "Scriptable Objects/PlayerState")]
public class PlayerState : ScriptableObject
{
    public string CharacterName; // キャラ名
    public float weight; // 重さ
    public float maxSpeed; // 最高速度
    public float moveForce; // 加速度
    public float jumpForce; // ジャンプ力
    public float rotationTorque; // 回転速度
    public float attack; // 攻撃力
    public float hp; // 体力
    public float ultCharge; // ウルトのチャージ量
}
