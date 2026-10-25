using UnityEngine;

public class hitsujiult : UltimateAbility
{
    public override void Execute(MovePlayer owner)
    {
        Rigidbody2D rb = owner.GetComponent<Rigidbody2D>();
        // rb.linearVelocity = new Vector2(rb.linearVelocity.x, 0f);
        float jumpForce = owner.playerState.jumpForce;
        rb.linearVelocity += (Vector2)(owner.transform.up * jumpForce);
    }
}
