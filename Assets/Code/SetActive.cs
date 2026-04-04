using UnityEngine;

public class SetActive : MonoBehaviour
{
    [SerializeField] private GameObject human;
    // Start is called once before the first execution of Update after the MonoBehaviour is created
    void Start()
    {
        human.SetActive(false);
    }
}
