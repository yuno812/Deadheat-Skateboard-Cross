using System.Collections;
using System.Collections.Generic;
using System.Linq;
using NUnit.Framework;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;
using UnityEngine.TestTools;
using Object = UnityEngine.Object;

namespace DscSim
{
    // P2 を CPU（CpuController + dsc_ai_native.dll）にして実際のゲームで試合を流し、
    // CPU が動いて、何もしない P1 にダメージを与えられるかを確かめる。
    public class CpuMatchTests
    {
        private const string Stage = "R2500";
        // 60 秒で打ち切り
        private const int MaxSteps = 3000;

        [UnityTest, Timeout(30 * 60 * 1000)]
        public IEnumerator CpuDamagesAnIdleOpponent()
        {
            yield return new EnterPlayMode();
            LogAssert.ignoreFailingMessages = true;

            Dictionary<string, SimProject.Character> characters = SimProject.SelectableCharacters().ToDictionary(c => c.Name);
            SimProject.EnsureManagers();
            var results = new List<string>();
            float captureDeltaTime = Time.captureDeltaTime;
            try
            {
                foreach ((string p1, string p2) in new[] { ("zero", "zero"), ("cubu", "okuda") })
                {
                    InputManager.Instance.inputP1 = new NullInputProvider();
                    InputManager.Instance.inputP2 = new CpuInputProvider();
                    PlayerSelection.Instance.playerPrefabP1 = characters[p1].Prefab;
                    PlayerSelection.Instance.playerPrefabP2 = characters[p2].Prefab;
                    EditorSceneManager.LoadSceneInPlayMode(SimProject.StageScenePath(Stage), new LoadSceneParameters(LoadSceneMode.Single));
                    Time.captureDeltaTime = Time.fixedDeltaTime;
                    yield return null;
                    yield return null;

                    // HP が 0 になってリザルト画面へ移ると、試合の後を見られないため
                    foreach (GameObserver observer in Object.FindObjectsByType<GameObserver>(FindObjectsSortMode.None))
                        observer.enabled = false;

                    MovePlayer[] players = Object.FindObjectsByType<MovePlayer>(FindObjectsSortMode.None).OrderBy(p => p.playerNumber).ToArray();
                    Assert.AreEqual(2, players.Length, "プレイヤーが2体生成されていません");
                    Assert.IsNotNull(players[1].GetComponent<CpuController>(), "P2 に CpuController が付いていません");
                    float startHp = players[0].HP;
                    double start = Time.fixedTimeAsDouble;
                    int pressed = 0;
                    while (players[0].HP > 0f && (Time.fixedTimeAsDouble - start) / Time.fixedDeltaTime < MaxSteps)
                    {
                        if (InputManager.Instance.inputP2.GetInput().jumpPressed) pressed++;
                        yield return null;
                    }
                    float seconds = (float)(Time.fixedTimeAsDouble - start);
                    string line = $"{p2} CPU vs {p1} idle: dealt {startHp - players[0].HP} in {seconds:F1}s (CPU hp {players[1].HP}, frames with jump pressed {pressed})";
                    Debug.Log($"[DscSim] {line}");
                    results.Add(line);
                    Assert.Greater(pressed, 0, $"{line}：CPU が入力を出していません");
                    Assert.Less(players[0].HP, startHp, $"{line}：CPU が1回もダメージを与えていません");
                }
            }
            finally
            {
                Time.captureDeltaTime = captureDeltaTime;
            }
            yield return new ExitPlayMode();
        }
    }
}
