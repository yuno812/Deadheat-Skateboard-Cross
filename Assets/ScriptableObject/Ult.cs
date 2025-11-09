using UnityEngine;

[CreateAssetMenu(fileName = "Ult", menuName = "Scriptable Objects/Ult")]
public class Ult : ScriptableObject
{
    public string ultName; //ウルト名 
    public string description; //ウルトの説明
    public float influence; // ダメージorスタン時間
    public float resource; //
}
