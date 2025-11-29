using UnityEngine;

public class GenerateBackground : MonoBehaviour
{
    [SerializeField] MoveBackgroud move;
    private int generatetime;
    public GameObject prefab;

    // Start is called once before the first execution of Update after the MonoBehaviour is created
    void Awake()
    {
        InvokeRepeating("Spawn", 0f, move.MoveTime);
    }

    void Spawn()
    {
        Instantiate(prefab, transform.position, transform.rotation);
    }
}
