using UnityEngine;

public class WheelGroundCheck : MonoBehaviour
{
    [SerializeField] private MovePlayer car;

    private void OnCollisionEnter2D(Collision2D collision)
    {
        if (collision.collider.CompareTag("Ground"))
            car.WheelLanded();
    }

    private void OnCollisionExit2D(Collision2D collision)
    {
        if (collision.collider.CompareTag("Ground"))
            car.WheelLifted();
    }

    private void OnTriggerEnter2D(Collider2D collision)
    {
        if (collision.CompareTag("Ground"))
            car.WheelLanded();
    }

    private void OnTriggerExit2D(Collider2D collision)
    {
        if (collision.CompareTag("Ground"))
            car.WheelLifted();
    }
}
