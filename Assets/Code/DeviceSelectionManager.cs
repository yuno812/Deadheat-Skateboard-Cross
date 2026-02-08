using UnityEngine;
using UnityEngine.SceneManagement;

public class DeviceSelectionManager : MonoBehaviour
{
    public DeviceSelector p1Keyboard, p1Controller;
    public DeviceSelector p2Keyboard, p2Controller;
    [SerializeField] private string nextSceneName;

    public void CheckAllPlayersReady()
    {
        bool p1Ready = (p1Keyboard.isSelected || p1Controller.isSelected);
        bool p2Ready = (p2Keyboard.isSelected || p2Controller.isSelected);

        if (p1Ready && p2Ready)
        {
            // 1. 選択された文字列を取得
            string p1Choice = p1Keyboard.isSelected ? "Keyboard" : "Controller";
            string p2Choice = p2Keyboard.isSelected ? "Keyboard" : "Controller";

            // 2. InputManagerに反映させる
            if (InputManager.Instance != null)
            {
                InputManager.Instance.ApplySelectedDevices(p1Choice, p2Choice);
            }

            // 3. 次のシーンへ
            SceneManager.LoadScene(nextSceneName);
        }
    }

    private void SaveDeviceSettings()
    {
        // 静的変数などに保存して次のシーンで参照できるようにします
        PlayerInputData.P1Device = p1Keyboard.isSelected ? "Keyboard" : "Controller";
        PlayerInputData.P2Device = p2Keyboard.isSelected ? "Keyboard" : "Controller";
    }
}

// データを保持するためのシンプルなクラス
public static class PlayerInputData
{
    public static string P1Device;
    public static string P2Device;
}