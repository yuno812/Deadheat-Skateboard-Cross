using UnityEngine;
using UnityEngine.InputSystem;
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
        currentIndex = 0;
        UpdateSelection();
    }

    void Update()
    {
        if (PlayerSelection.Instance == null) return;

        // キャラ選択中はステージ操作しない
        if (PlayerSelection.Instance.stageselect) return;

        var keyboard = Keyboard.current;
        if (keyboard == null) return;

        Vector2 move = Vector2.zero;

        // ===== キャラ選択と同じ入力 =====
        if (keyboard.wKey.wasPressedThisFrame || keyboard.upArrowKey.wasPressedThisFrame)
            move = Vector2.up;
        if (keyboard.sKey.wasPressedThisFrame || keyboard.downArrowKey.wasPressedThisFrame)
            move = Vector2.down;
        if (keyboard.aKey.wasPressedThisFrame || keyboard.leftArrowKey.wasPressedThisFrame)
            move = Vector2.left;
        if (keyboard.dKey.wasPressedThisFrame || keyboard.rightArrowKey.wasPressedThisFrame)
            move = Vector2.right;

        // ===== 移動 =====
        if (move != Vector2.zero && canMove)
        {
            canMove = false;
            MoveSelection(move);
            StartCoroutine(WaitMoveEnd());
        }

        // ===== 決定 =====
        if (keyboard.spaceKey.wasPressedThisFrame || keyboard.enterKey.wasPressedThisFrame)
        {
            // 選択中ステージのシーン名を保存
            PlayerSelection.Instance.nextSceneName =
                stages[currentIndex].SceneName;

            // キャラ選択へ
            PlayerSelection.Instance.stageselect = true;
        }
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

        // 存在しないマス対策（キャラ選択と同じ）
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
