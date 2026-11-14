using UnityEngine;

// CPU の入力。CpuController が物理ステップごとに決めた入力を、何度呼ばれても同じ値で返すだけ。
// メニュー画面などでは何も押さない（キャラ選択は SelectCharacter が CPU 用に自動で決める）。
public class CpuInputProvider : IInputProvider
{
    public InputState Current;

    public InputState GetInput()
    {
        return Current;
    }
}
