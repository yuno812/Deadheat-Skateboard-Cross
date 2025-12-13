using UnityEngine;

public class GameManager : MonoBehaviour
{
    public Transform spawnPointP1;
    public Transform spawnPointP2;
    public Transform outofArea;

    public HeartUI heartUI1; // 1P用 HeartUI
    public HeartUI heartUI2; // 2P用 HeartUI

    void Awake()
    {
        // PlayerSelection の存在チェック
        if (PlayerSelection.Instance == null)
        {
            Debug.LogError("PlayerSelection.Instance が null です！先に PlayerSelection オブジェクトをシーンに置いてください");
            return;
        }

        // プレイヤー1生成
        if (PlayerSelection.Instance.playerPrefabP1 != null)
        {
            GameObject player1 = Instantiate(PlayerSelection.Instance.playerPrefabP1, spawnPointP1.position, Quaternion.identity);
            MovePlayer mp1 = player1.GetComponent<MovePlayer>();
            mp1.playerNumber = 1;
            mp1.spawnArea = spawnPointP1.position;
            mp1.outofArea = outofArea.position;
        }
        else
        {
            Debug.LogError("playerPrefabP1 が未設定です！");
        }

        // プレイヤー2生成
        if (PlayerSelection.Instance.playerPrefabP2 != null)
        {
            GameObject player2 = Instantiate(PlayerSelection.Instance.playerPrefabP2, spawnPointP2.position, Quaternion.identity);
            MovePlayer mp2 = player2.GetComponent<MovePlayer>();
            mp2.playerNumber = 2;
            mp2.spawnArea = spawnPointP2.position;
            mp2.outofArea = outofArea.position;
        }
        else
        {
            Debug.LogError("playerPrefabP2 が未設定です！");
        }

        // -------------------------
        // HeartUI に heartPrefab を設定
        // -------------------------
        if (heartUI1 != null && PlayerSelection.Instance.heartPrefabP1 != null)
        {
            heartUI1.heartPrefab = PlayerSelection.Instance.heartPrefabP1;
            heartUI1.targetPlayerNumber = 1;
        }

        if (heartUI2 != null && PlayerSelection.Instance.heartPrefabP2 != null)
        {
            heartUI2.heartPrefab = PlayerSelection.Instance.heartPrefabP2;
            heartUI2.targetPlayerNumber = 2;
        }
    }
}
