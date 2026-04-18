using UnityEngine;

public class FollowPlayer : MonoBehaviour
{
    private Transform playerTransform;
    
    [Header("Offset")]
    [SerializeField] private float offsetX = 6f;
    [SerializeField] private float offsetY = 2.5f;

    void Start()
    {
        FindPlayer();
    }

    void Update()
    {
        // プレイヤーが見つかっていない場合は再度探す
        if (playerTransform == null)
        {
            FindPlayer();
            return;
        }

        // 追従処理
        transform.position = new Vector3(
            playerTransform.position.x + offsetX, 
            playerTransform.position.y + offsetY, 
            transform.position.z
        );
    }

    private void FindPlayer()
    {
        // シーン内の全 MovePlayer から 1P を探す
        MovePlayer[] players = Object.FindObjectsByType<MovePlayer>(FindObjectsSortMode.None);
        foreach (var p in players)
        {
            if (p.playerNumber == 1)
            {
                playerTransform = p.transform;
                break;
            }
        }
    }
}