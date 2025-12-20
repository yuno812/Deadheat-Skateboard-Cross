using UnityEngine;
// using UnityEngine.InputSystem;
using System.Collections;

public class SelectStage : MonoBehaviour
{
    [SerializeField] private StageIcon[] stages;
    [SerializeField] private int columns = 3;
    [SerializeField] private float moveDuration = 0.2f;

    private int currentIndex = 0;
    private bool canMove = true;

    void Start()
    {
        ResetState();
        UpdateSelection();
    }

    void Update()
    {
        if (PlayerSelection.Instance == null) return;
        // キャラ選択中はステージ操作しない
        if (PlayerSelection.Instance.stageselect) return;
        if (InputManager.Instance == null) return;

        var input1 = InputManager.Instance.inputP1.GetInput();
        var input2 = InputManager.Instance.inputP2.GetInput();

        Vector2 move = Vector2.zero;

        if (input1.move != Vector2.zero) move = input1.move;
        else if (input2.move != Vector2.zero) move = input2.move;

        // ===== 移動 =====
        if (move != Vector2.zero && canMove)
        {
            canMove = false;
            MoveSelection(move);
            StartCoroutine(WaitMoveEnd());
        }

        // ===== 決定 =====
        if (input1.confirm || input2.confirm)
        {
            // 選択中ステージのシーン名を保存
            PlayerSelection.Instance.nextSceneName =
                stages[currentIndex].SceneName;

            // キャラ選択へ
            PlayerSelection.Instance.stageselect = true;
        }
    }

    void ResetState()
    {
        currentIndex = 0;
        canMove = true;
    }

    void MoveSelection(Vector2 dir)
    {
        int row = currentIndex / columns;
        int col = currentIndex % columns;
        int rowCount = Mathf.CeilToInt((float)stages.Length / columns);

        if (dir == Vector2.up) row = (row - 1 + rowCount) % rowCount;
        if (dir == Vector2.down) row = (row + 1) % rowCount;
        if (dir == Vector2.left) col = (col - 1 + columns) % columns;
        if (dir == Vector2.right) col = (col + 1) % columns;

        int newIndex = row * columns + col;

        // キャラ選択と同じ「存在しないマス対策」
        if (newIndex >= stages.Length)
            newIndex = row * columns;

        currentIndex = newIndex;
        UpdateSelection();
    }


    void UpdateSelection()
    {
        for (int i = 0; i < stages.Length; i++)
        {
            stages[i].SetSelected(i == currentIndex);
        }
    }

    IEnumerator WaitMoveEnd()
    {
        yield return new WaitForSeconds(moveDuration);
        canMove = true;
    }
}
