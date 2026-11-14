using System.Collections;
using System.Collections.Generic;
using System.Linq;
using NUnit.Framework;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;
using UnityEngine.TestTools;
using Dsc.Ai.Native;
using Object = UnityEngine.Object;

namespace DscSim
{
    // P2 を CPU（CpuController + dsc_ai_native.dll）にして実際のゲームで試合を流し、
    // CPU が動いて、何もしない P1 にダメージを与えられるかを確かめる。
    // 試合を何本も流す（数分かかる）ので、Test Runner の Run All では動かさない。
    // 名前（DscSim.CpuMatchTests など）を指定して流す。手順は rust/docs/CPU調整ガイド.md
    [Explicit("CPU の試合を流す（数分）。名前を指定して流す")]
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
            // CPU の判断そのものを見るので、ハンデの無い「つよい」で流す
            InputManager.Instance.cpuDifficulty = Difficulty.Strong;
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

        // 重力反転のあるステージ（Rugby：y > 0 で重力が上向き。StageGravity）で、CPU が止まらずに戦えるか。
        // 0.5 秒ごとの CPU の状態を記録し、最後の 10 秒でほとんど動いていなければ「止まった」とみなす
        [UnityTest, Timeout(30 * 60 * 1000)]
        public IEnumerator CpuKeepsPlayingWithReversedGravity()
        {
            yield return new EnterPlayMode();
            LogAssert.ignoreFailingMessages = true;

            Dictionary<string, SimProject.Character> characters = SimProject.SelectableCharacters().ToDictionary(c => c.Name);
            SimProject.EnsureManagers();
            // CPU の判断そのものを見るので、ハンデの無い「つよい」で流す
            InputManager.Instance.cpuDifficulty = Difficulty.Strong;
            float captureDeltaTime = Time.captureDeltaTime;
            var failures = new List<string>();
            try
            {
                foreach (string cpu in new[] { "zero", "cubu", "hitsuji" })
                {
                    InputManager.Instance.inputP1 = new NullInputProvider();
                    InputManager.Instance.inputP2 = new CpuInputProvider();
                    PlayerSelection.Instance.playerPrefabP1 = characters["zero"].Prefab;
                    PlayerSelection.Instance.playerPrefabP2 = characters[cpu].Prefab;
                    EditorSceneManager.LoadSceneInPlayMode(SimProject.StageScenePath("Rugby"), new LoadSceneParameters(LoadSceneMode.Single));
                    Time.captureDeltaTime = Time.fixedDeltaTime;
                    yield return null;
                    yield return null;
                    foreach (GameObserver observer in Object.FindObjectsByType<GameObserver>(FindObjectsSortMode.None))
                        observer.enabled = false;

                    MovePlayer[] players = Object.FindObjectsByType<MovePlayer>(FindObjectsSortMode.None).OrderBy(p => p.playerNumber).ToArray();
                    Assert.AreEqual(2, players.Length, "プレイヤーが2体生成されていません");
                    MovePlayer me = players[1];
                    Rigidbody2D body = me.GetComponent<Rigidbody2D>();
                    float startHp = players[0].HP;
                    double start = Time.fixedTimeAsDouble;
                    double nextSample = 0;
                    int upperSamples = 0, samples = 0;
                    var path = new List<Vector2>();
                    while ((Time.fixedTimeAsDouble - start) < 40.0 && me.HP > 0f && players[0].HP > 0f)
                    {
                        double t = Time.fixedTimeAsDouble - start;
                        if (t >= nextSample)
                        {
                            nextSample += 0.5;
                            InputState input = InputManager.Instance.inputP2.GetInput();
                            path.Add(body.position);
                            samples++;
                            if (body.position.y > 0f) upperSamples++;
                            Debug.Log($"[DscSim] rugby {cpu} t={t:F1} pos=({body.position.x:F2},{body.position.y:F2}) angle={Vector2.SignedAngle(Vector2.up, me.transform.up):F0} " +
                                      $"ground={me.GroundWheelCount} gscale={body.gravityScale} input=(move {input.moveX}, rot {input.rotate}, jump {input.jumpPressed}) opp=({players[0].transform.position.x:F2},{players[0].transform.position.y:F2})");
                        }
                        yield return null;
                    }
                    float seconds = (float)(Time.fixedTimeAsDouble - start);
                    // 最後の 10 秒（20 サンプル）で動いた範囲
                    float spread = 0f;
                    for (int i = Mathf.Max(0, path.Count - 20); i < path.Count; i++)
                        spread = Mathf.Max(spread, Vector2.Distance(path[i], path[path.Count - 1]));
                    string line = $"rugby {cpu} CPU vs zero idle: dealt {startHp - players[0].HP} in {seconds:F1}s, upper half {upperSamples}/{samples} samples, last 10s spread {spread:F2}";
                    Debug.Log($"[DscSim] {line}");
                    if (players[0].HP > 0f && spread < 0.5f) failures.Add(line);
                }
            }
            finally
            {
                Time.captureDeltaTime = captureDeltaTime;
            }
            yield return new ExitPlayMode();
            Assert.IsEmpty(failures, "CPU が止まっています");
        }

        // 難易度ごとに、ゲームの中で CPU（zero）が何もしない zero にダメージを与えられるか。強さの差は E6（シミュレーター）で測る
        [UnityTest, Timeout(30 * 60 * 1000)]
        public IEnumerator EveryDifficultyPlays()
        {
            yield return new EnterPlayMode();
            LogAssert.ignoreFailingMessages = true;

            Dictionary<string, SimProject.Character> characters = SimProject.SelectableCharacters().ToDictionary(c => c.Name);
            SimProject.EnsureManagers();
            float captureDeltaTime = Time.captureDeltaTime;
            var failures = new List<string>();
            try
            {
                foreach (Difficulty difficulty in new[] { Difficulty.Weak, Difficulty.Normal, Difficulty.Strong })
                {
                    InputManager.Instance.inputP1 = new NullInputProvider();
                    InputManager.Instance.inputP2 = new CpuInputProvider();
                    InputManager.Instance.cpuDifficulty = difficulty;
                    PlayerSelection.Instance.playerPrefabP1 = characters["zero"].Prefab;
                    PlayerSelection.Instance.playerPrefabP2 = characters["zero"].Prefab;
                    EditorSceneManager.LoadSceneInPlayMode(SimProject.StageScenePath(Stage), new LoadSceneParameters(LoadSceneMode.Single));
                    Time.captureDeltaTime = Time.fixedDeltaTime;
                    yield return null;
                    yield return null;
                    foreach (GameObserver observer in Object.FindObjectsByType<GameObserver>(FindObjectsSortMode.None))
                        observer.enabled = false;

                    MovePlayer[] players = Object.FindObjectsByType<MovePlayer>(FindObjectsSortMode.None).OrderBy(p => p.playerNumber).ToArray();
                    Assert.AreEqual(2, players.Length, "プレイヤーが2体生成されていません");
                    CpuController cpu = players[1].GetComponent<CpuController>();
                    Assert.AreEqual(difficulty, cpu.difficulty, "難易度が CpuController に渡っていません");
                    Assert.AreEqual(difficulty == Difficulty.Strong, cpu.reactionDelaySteps == 0, "ハンデの値が CpuController に入っていません");
                    // プレイ中にインスペクターで変えたハンデが、その場で CPU に届く
                    if (difficulty == Difficulty.Weak)
                    {
                        LogAssert.Expect(LogType.Log, new System.Text.RegularExpressions.Regex("ハンデを変更.*もたつき 0.5"));
                        cpu.slip = 0.5f;
                    }
                    float startHp = players[0].HP;
                    double start = Time.fixedTimeAsDouble;
                    while (players[0].HP > 0f && (Time.fixedTimeAsDouble - start) / Time.fixedDeltaTime < MaxSteps)
                        yield return null;
                    string line = $"{difficulty} zero CPU vs zero idle: dealt {startHp - players[0].HP} in {Time.fixedTimeAsDouble - start:F1}s (CPU hp {players[1].HP})";
                    Debug.Log($"[DscSim] {line}");
                    if (players[0].HP >= startHp) failures.Add(line);
                }
            }
            finally
            {
                Time.captureDeltaTime = captureDeltaTime;
            }
            yield return new ExitPlayMode();
            Assert.IsEmpty(failures, "CPU が1回もダメージを与えていません");
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
            // CPU の判断そのものを見るので、ハンデの無い「つよい」で流す
            InputManager.Instance.cpuDifficulty = Difficulty.Strong;
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
