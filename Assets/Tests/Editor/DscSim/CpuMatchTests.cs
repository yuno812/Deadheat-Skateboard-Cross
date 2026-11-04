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

        // EnterPlayMode でドメインが読み直されるため、テストのローカル変数をラムダで捕まえると消える。数は static に持つ
        private static int ultFired;

        private static void CountUlt(string message, string stack, LogType type)
        {
            if (message.StartsWith("2P: ウルト")) ultFired++;
        }

        // ウルトを撃つ4キャラを CPU にし、何もしない zero を相手に 40 秒流して、撃った回数を数える。
        // すぐ撃つキャラ（cubu・maiden・meme）は、ゲージが溜まれば必ず撃つはず。hitsuji は場面次第なので数を出すだけ
        [UnityTest, Timeout(30 * 60 * 1000)]
        public IEnumerator CpuFiresItsUltimate()
        {
            yield return new EnterPlayMode();
            LogAssert.ignoreFailingMessages = true;

            Dictionary<string, SimProject.Character> characters = SimProject.SelectableCharacters().ToDictionary(c => c.Name);
            SimProject.EnsureManagers();
            float captureDeltaTime = Time.captureDeltaTime;
            Application.logMessageReceived += CountUlt;
            var failures = new List<string>();
            try
            {
                foreach ((string cpu, bool mustFire) in new[] { ("cubu", true), ("maiden", true), ("meme", true), ("hitsuji", false) })
                {
                    InputManager.Instance.inputP1 = new NullInputProvider();
                    InputManager.Instance.inputP2 = new CpuInputProvider();
                    PlayerSelection.Instance.playerPrefabP1 = characters["zero"].Prefab;
                    PlayerSelection.Instance.playerPrefabP2 = characters[cpu].Prefab;
                    EditorSceneManager.LoadSceneInPlayMode(SimProject.StageScenePath(Stage), new LoadSceneParameters(LoadSceneMode.Single));
                    Time.captureDeltaTime = Time.fixedDeltaTime;
                    yield return null;
                    yield return null;
                    foreach (GameObserver observer in Object.FindObjectsByType<GameObserver>(FindObjectsSortMode.None))
                        observer.enabled = false;

                    MovePlayer[] players = Object.FindObjectsByType<MovePlayer>(FindObjectsSortMode.None).OrderBy(p => p.playerNumber).ToArray();
                    Assert.AreEqual(2, players.Length, "プレイヤーが2体生成されていません");
                    ultFired = 0;
                    float startHp = players[0].HP;
                    float maxGauge = 0f;
                    double start = Time.fixedTimeAsDouble;
                    // 相手の HP が 0 になってもゲージは溜まるので、時間いっぱい流す
                    while ((Time.fixedTimeAsDouble - start) < 40.0 && players[1].HP > 0f)
                    {
                        maxGauge = Mathf.Max(maxGauge, players[1].ultGauge);
                        yield return null;
                    }
                    float seconds = (float)(Time.fixedTimeAsDouble - start);
                    string line = $"{cpu} CPU vs zero idle: ult fired {ultFired} times in {seconds:F1}s (gauge max {maxGauge:F1}/{players[1].ultAmount}, dealt {startHp - players[0].HP}, CPU hp {players[1].HP})";
                    Debug.Log($"[DscSim] {line}");
                    if (mustFire && ultFired == 0) failures.Add(line);
                }
            }
            finally
            {
                Application.logMessageReceived -= CountUlt;
                Time.captureDeltaTime = captureDeltaTime;
            }
            yield return new ExitPlayMode();
            Assert.IsEmpty(failures, "ウルトを1回も撃っていません");
        }
    }
}
