using UnityEngine;
using UnityEngine.SceneManagement;

public class GameObserver : MonoBehaviour
{
    private MovePlayer Player1;
    private MovePlayer Player2;
    private bool lose1 = false;
    private bool lose2 = false;

    // Start is called once before the first execution of Update after the MonoBehaviour is created
    void Start()
    {
        GameObject[] players = GameObject.FindGameObjectsWithTag("Player");
        foreach (var obj in players)
        {
            MovePlayer mp = obj.GetComponent<MovePlayer>();
            if (mp != null && mp.playerNumber == 1)
            {
                Player1 = mp;
            }
            else if (mp != null && mp.playerNumber == 2)
            {
                Player2 = mp;
            }
        }
    }

    // Update is called once per frame
    void Update()
    {
        if(Player1.HP <= 0)
        {
            lose1 = true;
        }
        if(Player2.HP <= 0)
        {
            lose2 = true;
        }

        if (lose1 && lose2)
        {
            SceneManager.LoadScene("ResultScene");
        }
        else if (lose1)
        {
            SceneManager.LoadScene("ResultScene");
        }
        else if (lose2)
        {
            SceneManager.LoadScene("ResultScene");
        }

    }
}
