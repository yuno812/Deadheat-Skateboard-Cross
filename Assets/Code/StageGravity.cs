using UnityEngine;

public class StageGravity : MonoBehaviour
{
    private MovePlayer[] players;

    void Start()
    {
        players = FindObjectsByType<MovePlayer>(FindObjectsSortMode.None);
    }

    void FixedUpdate()
    {
        foreach (MovePlayer player in players)
        {
            Rigidbody2D rb = player.GetComponent<Rigidbody2D>();

            if (player.transform.position.y > 0)
            {
                rb.gravityScale = -1f;
            }
            else
            {
                rb.gravityScale = 1f;
            }
        }
    }
}