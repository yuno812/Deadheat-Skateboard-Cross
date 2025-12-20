using UnityEngine;
using UnityEngine.InputSystem;
using System.Collections;
using UnityEngine.SceneManagement;

public class SelectCharacter : MonoBehaviour
{
    [SerializeField] private CharacterIcon[] characters;
    [SerializeField] private int columns = 3;
    [SerializeField] private float moveDuration = 0.2f;

    private int currentIndexP1;
    private int currentIndexP2;

    private bool canMoveP1;
    private bool canMoveP2;

    private bool confirmedP1;
    private bool confirmedP2;

    private bool countdownStarted;
    private bool lastStageSelect;

    // ★ 追加：入力ロック
    private bool inputLocked;

    void Start()
    {
        ResetState();
        UpdateSelection();
    }

    void Update()
    {
        if (PlayerSelection.Instance == null) return;
        if (InputManager.Instance == null) return;

        bool stageSelect = PlayerSelection.Instance.stageselect;

        // ===== ステージ選択 → キャラ選択に入った瞬間 =====
        if (stageSelect && !lastStageSelect)
        {
            ResetState();
            UpdateSelection();

            inputLocked = true;
            StartCoroutine(UnlockInputNextFrame());
        }
        lastStageSelect = stageSelect;

        if (!stageSelect) return;
        if (inputLocked) return; // ★ ここが重要

        var input1 = InputManager.Instance.inputP1.GetInput();
        var input2 = InputManager.Instance.inputP2.GetInput();

        // ===== ESC：ステージ選択へ戻る =====
        if ((input1.cancel || input2.cancel) && stageSelect)
        {
            PlayerSelection.Instance.stageselect = false;
            ResetState();
            UpdateSelection();
            StopAllCoroutines();
            return;
        }

        // -------- 1P --------
        if (!confirmedP1 && canMoveP1 && input1.move != Vector2.zero)
        {
            canMoveP1 = false;
            MoveSelection(ref currentIndexP1, input1.move);
            UpdateSelection();
            StartCoroutine(WaitMoveEndP1());
        }

        if (input1.confirm)
        {
            confirmedP1 = !confirmedP1;
            if (confirmedP1)
            {
                PlayerSelection.Instance.playerPrefabP1 = characters[currentIndexP1].PlayerPrefab;
                PlayerSelection.Instance.heartPrefabP1 = characters[currentIndexP1].HeartPrefab;
            }
            UpdateSelection();
        }

        // -------- 2P --------
        if (!confirmedP2 && canMoveP2 && input2.move != Vector2.zero)
        {
            canMoveP2 = false;
            MoveSelection(ref currentIndexP2, input2.move);
            UpdateSelection();
            StartCoroutine(WaitMoveEndP2());
        }

        if (input2.confirm)
        {
            confirmedP2 = !confirmedP2;
            if (confirmedP2)
            {
                PlayerSelection.Instance.playerPrefabP2 = characters[currentIndexP2].PlayerPrefab;
                PlayerSelection.Instance.heartPrefabP2 = characters[currentIndexP2].HeartPrefab;
            }
            UpdateSelection();
        }

        // -------- 両者確定 --------
        if (confirmedP1 && confirmedP2 && !countdownStarted)
        {
            countdownStarted = true;
            StartCoroutine(StartCountdown());
        }
    }

    // =========================
    // 共通処理
    // =========================

    void ResetState()
    {
        currentIndexP1 = 0;
        currentIndexP2 = 0;
        confirmedP1 = false;
        confirmedP2 = false;
        canMoveP1 = true;
        canMoveP2 = true;
        countdownStarted = false;
    }

    IEnumerator UnlockInputNextFrame()
    {
        yield return null; // ★ 1フレーム待つ
        inputLocked = false;
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
        if (newIndex >= characters.Length)
            newIndex = row * columns;

        currentIndex = newIndex;
    }

    void UpdateSelection()
    {
        for (int i = 0; i < characters.Length; i++)
        {
            characters[i].SetSelected(
                i == currentIndexP1,
                i == currentIndexP2,
                confirmedP1,
                confirmedP2
            );
        }
    }

    IEnumerator WaitMoveEndP1()
    {
        yield return new WaitForSeconds(moveDuration);
        canMoveP1 = true;
    }

    IEnumerator WaitMoveEndP2()
    {
        yield return new WaitForSeconds(moveDuration);
        canMoveP2 = true;
    }

    IEnumerator StartCountdown()
    {
        yield return new WaitForSeconds(3f);
        SceneManager.LoadScene(PlayerSelection.Instance.nextSceneName);
    }
}
