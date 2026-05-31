using UnityEngine;
using UnityEngine.UI;
using UnityEngine.SceneManagement;

public class TitleLogo : MonoBehaviour
{
    int Step = 1;
    public GameObject Logo;
    float a;
    void Start()
    {
        Invoke("Next", 3f);
        Invoke("Next", 4.5f);
    }
    void Update()
    {
        if(Step == 1 && a < 1f)
        {
            a += Time.deltaTime;
        }
        if (Step == 2)
        {
            a -= Time.deltaTime;
        }
        if (Step == 3)
        {
            SceneManager.LoadScene("TitleScene");
        }
        Logo.GetComponent<Image>().color = new Color(1f, 1f, 1f, a);
    }
    void Next()
    {
        Step += 1;
    }
}