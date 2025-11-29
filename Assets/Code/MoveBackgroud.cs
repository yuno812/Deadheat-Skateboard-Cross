using UnityEngine;

public class MoveBackgroud : MonoBehaviour
{
    public float MoveTime = 4f;

    void Update()
    {
        transform.position += new Vector3(0, Time.deltaTime/MoveTime * 12, 0);
        if (transform.position.y >= 12)
        {
            Destroy(this.gameObject);
        }
    }
}
