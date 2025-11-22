using UnityEngine;
using UnityEngine.InputSystem;
using System.Collections;
using UnityEngine.SceneManagement;

public class SelectCharacter : MonoBehaviour
{
    [SerializeField] private CharacterIcon[] characters;
    [SerializeField] private int columns = 3;
    [SerializeField] private string nextSceneName = "GameScene";

    private int currentIndexP1 = 0;
    private int currentIndexP2 = 0;

    private bool canMoveP1 = true;
    private bool canMoveP2 = true;

    private bool confirmedP1 = false;
    private bool confirmedP2 = false;

    private bool countdownStarted = false;

    void Start()
    {
        UpdateSelection();
    }

    void Update()
    {
        var keyboard = Keyboard.current;
        if (keyboard == null) return;

        // -------- 1P入力 --------
        Vector2 moveP1 = Vector2.zero;
        if (!confirmedP1)
        {
            if (keyboard.wKey.wasPressedThisFrame) moveP1 = Vector2.up;
            if (keyboard.sKey.wasPressedThisFrame) moveP1 = Vector2.down;
            if (keyboard.aKey.wasPressedThisFrame) moveP1 = Vector2.left;
            if (keyboard.dKey.wasPressedThisFrame) moveP1 = Vector2.right;

            if (moveP1 != Vector2.zero && canMoveP1)
            {
                MoveSelection(ref currentIndexP1, moveP1);
                canMoveP1 = false;
                Invoke(nameof(ResetMoveFlagP1), 0.15f);
            }
        }

        if (keyboard.spaceKey.wasPressedThisFrame)
        {
            confirmedP1 = !confirmedP1;
            if (confirmedP1)
            {
                PlayerSelection.Instance.playerPrefabP1 = characters[currentIndexP1].PlayerPrefab;
                PlayerSelection.Instance.heartPrefabP1 = characters[currentIndexP1].HeartPrefab;
            }
        }

        // -------- 2P入力 --------
        Vector2 moveP2 = Vector2.zero;
        if (!confirmedP2)
        {
            if (keyboard.upArrowKey.wasPressedThisFrame) moveP2 = Vector2.up;
            if (keyboard.downArrowKey.wasPressedThisFrame) moveP2 = Vector2.down;
            if (keyboard.leftArrowKey.wasPressedThisFrame) moveP2 = Vector2.left;
            if (keyboard.rightArrowKey.wasPressedThisFrame) moveP2 = Vector2.right;

            if (moveP2 != Vector2.zero && canMoveP2)
            {
                MoveSelection(ref currentIndexP2, moveP2);
                canMoveP2 = false;
                Invoke(nameof(ResetMoveFlagP2), 0.15f);
            }
        }

        if (keyboard.enterKey.wasPressedThisFrame)
        {
            confirmedP2 = !confirmedP2;
            if (confirmedP2)
            {
                PlayerSelection.Instance.playerPrefabP2 = characters[currentIndexP2].PlayerPrefab;
                PlayerSelection.Instance.heartPrefabP2 = characters[currentIndexP2].HeartPrefab;
            }
        }

        // -------- 両方決定でカウント開始 --------
        if (confirmedP1 && confirmedP2 && !countdownStarted)
        {
            countdownStarted = true;
            StartCoroutine(StartCountdown());
        }

        // どちらかキャンセルしたらカウント中止
        if ((!confirmedP1 || !confirmedP2) && countdownStarted)
        {
            countdownStarted = false;
            StopAllCoroutines();
        }

        UpdateSelection();
    }

    void MoveSelection(ref int currentIndex, Vector2 dir)
    {
        int row = currentIndex / columns;
        int col = currentIndex % columns;
        int rowCount = Mathf.CeilToInt((float)characters.Length / columns);

        if (dir == Vector2.up) row = (row - 1 + rowCount) % rowCount;
        if (dir == Vector2.down) row = (row + 1) % rowCount;
        if (dir == Vector2.left) col = (col - 1 + columns) % columns;
        if (dir == Vector2.right) col = (col + 1) % columns;

        int newIndex = row * columns + col;
        if (newIndex >= characters.Length) newIndex = row * columns;

        currentIndex = newIndex;
    }

    void UpdateSelection()
    {
        for (int i = 0; i < characters.Length; i++)
        {
            bool p1Cursor = (i == currentIndexP1);
            bool p2Cursor = (i == currentIndexP2);

            // 確定状態も渡す
            characters[i].SetSelected(p1Cursor, p2Cursor, confirmedP1, confirmedP2);
        }
    }

    private IEnumerator StartCountdown()
    {
        float timer = 3f;
        while (timer > 0)
        {
            timer -= Time.deltaTime;
            yield return null;
        }
        SceneManager.LoadScene(nextSceneName);
    }

    void ResetMoveFlagP1() => canMoveP1 = true;
    void ResetMoveFlagP2() => canMoveP2 = true;
}
