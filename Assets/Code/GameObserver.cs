using UnityEngine;
using UnityEngine.SceneManagement;
using System.Collections;

public class GameObserver : MonoBehaviour
{
    private MovePlayer Player1;
    private MovePlayer Player2;
    private bool lose1 = false;
    private bool lose2 = false;
    [SerializeField] private GameObject Game;

    // Start is called once before the first execution of Update after the MonoBehaviour is created
    void Start()
    {
        Game.SetActive(false);
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
            PlayerSelection.Instance.lose1 = true;
            PlayerSelection.Instance.lose2 = true;
            StartCoroutine(FinishGame());
        }
        else if (lose1)
        {
            PlayerSelection.Instance.lose1 = true;
            StartCoroutine(FinishGame());
        }
        else if (lose2)
        {
            PlayerSelection.Instance.lose2 = true;
            StartCoroutine(FinishGame());
        }

    }

    private IEnumerator FinishGame()
    {
        Game.SetActive(true);
        Time.timeScale = 0.3f;
        yield return new WaitForSeconds(0.45f);
        Time.timeScale = 1.0f;
        SceneManager.LoadScene("ResultScene");
    }
}
