using UnityEngine;
using Unity.Cinemachine; 

public class FollowPlayer : MonoBehaviour
{
    private Transform playerTransform;

    void Start()
    {
        FindPlayer();
        this.GetComponent<CinemachineCamera>().Follow = playerTransform;
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