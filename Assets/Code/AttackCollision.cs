using UnityEngine;

public class AttackCollision : MonoBehaviour
{
    [SerializeField] private MovePlayer parent;

    void OnCollisionEnter2D(Collision2D collision)
    {
        // 相手の Collider を collision.collider から取得
        GameObject other = collision.collider.gameObject;

        if (other.CompareTag("MainCollision"))
        {
            parent.HitAttack(other, false);
        }
        else if (other.CompareTag("AttackCollision"))
        {
            parent.HitSame(other);
        }
    }
}
