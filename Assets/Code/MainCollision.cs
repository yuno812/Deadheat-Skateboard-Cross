using UnityEngine;

public class MainCollision : MonoBehaviour
{
    [SerializeField] private MovePlayer parent;

    void OnCollisionEnter2D(Collision2D collision)
    {
        // 衝突した相手の GameObject を取得
        GameObject other = collision.collider.gameObject;

        if (other.CompareTag("MainCollision"))
        {
            parent.HitSame(other);
        }
        else if (other.CompareTag("AttackCollision"))
        {
            parent.HitAttack(other, true);
        }
    }
}
