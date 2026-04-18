using UnityEngine;
using UnityEngine.SceneManagement;

public class TDeviceSelectionManager : MonoBehaviour
{
    public DeviceSelector p1Keyboard, p1Controller;

    [SerializeField] private GameObject playerPrefabP1;
    [SerializeField] private GameObject heartPrefabP1;
    [SerializeField] private Sprite resultHeader1;
    [SerializeField] private Sprite Icon1;
    [SerializeField] private GameObject playerPrefabP2;
    [SerializeField] private GameObject heartPrefabP2;
    [SerializeField] private Sprite resultHeader2;
    [SerializeField] private Sprite Icon2;
    [SerializeField] private string nextSceneName;

    public void CheckAllPlayersReady()
    {
        bool p1Ready = (p1Keyboard.isSelected || p1Controller.isSelected);

        if (p1Ready)
        {
            SaveDeviceSettings();

            // 1. 選択された文字列を取得
            string p1Choice = p1Keyboard.isSelected ? "Keyboard" : "Controller";
            string p2Choice = "null";

            // 2. InputManagerに反映させる
            if (InputManager.Instance != null)
            {
                InputManager.Instance.ApplySelectedDevices(p1Choice, p2Choice);
            }

            PlayerSelection.Instance.playerPrefabP1 = playerPrefabP1;
            PlayerSelection.Instance.heartPrefabP1 = heartPrefabP1;
            PlayerSelection.Instance.resultHeader1 = resultHeader1;
            PlayerSelection.Instance.Icon1 = Icon1;
            PlayerSelection.Instance.playerPrefabP2 = playerPrefabP2;
            PlayerSelection.Instance.heartPrefabP2 = heartPrefabP2;
            PlayerSelection.Instance.resultHeader2 = resultHeader2;
            PlayerSelection.Instance.Icon2 = Icon2;
            // 3. 次のシーンへ
            SceneManager.LoadScene(nextSceneName);
        }
    }

    private void SaveDeviceSettings()
    {
        // 静的変数などに保存して次のシーンで参照できるようにします
        PlayerInputData.P1Device = p1Keyboard.isSelected ? "Keyboard" : "Controller";
        PlayerInputData.P2Device = "null";
    }
}

// データを保持するためのシンプルなクラス
public static class TPlayerInputData
{
    public static string P1Device;
    public static string P2Device;
}