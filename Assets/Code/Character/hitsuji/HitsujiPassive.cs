using UnityEngine;

public class HitsujiPassive : MonoBehaviour
{
    [SerializeField]private MovePlayer player;

    void Update()
    {
        if (player != null && player.HP <= 1)
        {
            player.ultAmount = 0.1f;
        }
    }
}
