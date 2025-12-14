using UnityEngine;

public class SelectMove : MonoBehaviour
{
    public Vector3 pointA;
    public Vector3 pointB;
    public float moveTime = 2f; // A→Bにかかる時間

    float moveSpeed;

    void Start()
    {
        transform.position = pointA;

        // A→B を moveTime 秒で移動する速度
        moveSpeed = Vector3.Distance(pointA, pointB) / moveTime;
    }

    void Update()
    {
        if (PlayerSelection.Instance == null) return;

        Vector3 target =
            PlayerSelection.Instance.stageselect ? pointB : pointA;

        transform.position = Vector3.MoveTowards(
            transform.position,
            target,
            moveSpeed * Time.deltaTime
        );
    }
}
