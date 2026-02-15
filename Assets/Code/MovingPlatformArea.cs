using UnityEngine;

public class MovingPlatformArea : MonoBehaviour
{
    // エリアに入った時
    private void OnTriggerEnter2D(Collider2D other)
    {
        var player = other.GetComponentInParent<MovePlayer>();
        if (player != null)
        {
            player.WheelLanded();
        }
    }

    // エリアから出た時
    private void OnTriggerExit2D(Collider2D other)
    {
        var player = other.GetComponentInParent<MovePlayer>();
        if (player != null)
        {
            player.WheelLifted();
        }
    }
}