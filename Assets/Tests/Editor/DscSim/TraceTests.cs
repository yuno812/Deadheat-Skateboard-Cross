using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text;
using System.Text.RegularExpressions;
using NUnit.Framework;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;
using UnityEngine.TestTools;
using Object = UnityEngine.Object;

namespace DscSim
{
    // シナリオを実際のゲームで再生し、毎ステップの状態をトレースCSVに記録する（照合の「正解」）。
    // ゲームのコードやシーンは変更しない。入力は ScriptedInputProvider で差し込む。実行は scripts/trace-unity.ps1。
    //
    // 環境変数：DSC_SCENARIOS（既定 rust/data/scenarios、下のフォルダも含む）、
    //           DSC_TRACE_OUT（既定 rust/data/traces/unity）、DSC_SCENARIO_FILTER（idの正規表現）
    public class TraceTests
    {
        private static readonly FieldInfo LastPosition =
            typeof(MovePlayer).GetField("lastPosition", BindingFlags.Instance | BindingFlags.NonPublic);

        // 全シナリオを1件のテストで流すので、既定の3分では足りない（R1 は数百本ある）
        [UnityTest, Timeout(6 * 60 * 60 * 1000)]
        public IEnumerator Trace()
        {
            yield return new EnterPlayMode();

            // シーン内のUIなどがエラーを出しても、記録は続ける
            LogAssert.ignoreFailingMessages = true;

            string outDir = Env("DSC_TRACE_OUT", Path.Combine(SimProject.DataDir(), "traces", "unity"));
            Directory.CreateDirectory(outDir);
            List<Scenario> scenarios = LoadScenarios();
            Assert.IsNotEmpty(scenarios, "再生するシナリオがありません");
            Dictionary<string, SimProject.Character> characters =
                SimProject.SelectableCharacters().ToDictionary(c => c.Name);
            SimProject.EnsureManagers();

            var failures = new List<string>();
            float timeScale = Time.timeScale;
            float captureDeltaTime = Time.captureDeltaTime;
            Time.timeScale = 0f;
            try
            {
                foreach (Scenario scenario in scenarios)
                {
                    if (!characters.TryGetValue(scenario.P1, out var c1) || !characters.TryGetValue(scenario.P2, out var c2))
                    {
                        failures.Add($"{scenario.Id}: 不明なキャラ {scenario.P1} / {scenario.P2}");
                        continue;
                    }

                    // 時間を止めたままステージを読み込み、Awake/Start だけを走らせる
                    PlayerSelection.Instance.playerPrefabP1 = c1.Prefab;
                    PlayerSelection.Instance.playerPrefabP2 = c2.Prefab;
                    EditorSceneManager.LoadSceneInPlayMode(SimProject.StageScenePath(scenario.Stage),
                        new LoadSceneParameters(LoadSceneMode.Single));
                    yield return null;
                    yield return null;

                    TraceRecorder recorder = Prepare(scenario, out string error);
                    if (recorder == null)
                    {
                        failures.Add($"{scenario.Id}: {error}");
                        continue;
                    }

                    // 1フレームにつき物理ステップがちょうど1回進み、実時間を待たずに最速で回る
                    Time.captureDeltaTime = Time.fixedDeltaTime;
                    Time.timeScale = 1f;
                    int frames = 0;
                    while (!recorder.Done && recorder.Error == null && frames++ < scenario.Steps * 4 + 100)
                        yield return null;
                    Time.timeScale = 0f;
                    InputManager.Instance.StopAllCoroutines();

                    if (recorder.Done)
                        File.WriteAllText(Path.Combine(outDir, scenario.Id + ".csv"), recorder.Csv, new UTF8Encoding(false));
                    else
                        failures.Add($"{scenario.Id}: {recorder.Error ?? "時間切れ"}");
                }
            }
            finally
            {
                Time.timeScale = timeScale;
                Time.captureDeltaTime = captureDeltaTime;
            }

            yield return new ExitPlayMode();
            Debug.Log($"[DscSim] traced {scenarios.Count - failures.Count}/{scenarios.Count} scenarios to {outDir}");
            Assert.IsEmpty(failures, string.Join("\n", failures));
        }

        private static List<Scenario> LoadScenarios()
        {
            string dir = Env("DSC_SCENARIOS", Path.Combine(SimProject.DataDir(), "scenarios"));
            string filter = Environment.GetEnvironmentVariable("DSC_SCENARIO_FILTER");
            return Directory.GetFiles(dir, "*.json", SearchOption.AllDirectories)
                .OrderBy(f => f, StringComparer.Ordinal)
                .Select(f => Scenario.Parse(File.ReadAllText(f)))
                .Where(s => string.IsNullOrEmpty(filter) || Regex.IsMatch(s.Id, filter))
                .ToList();
        }

        // 入力の差し込み、GameObserver の停止、init の反映をして、記録係を返す
        private static TraceRecorder Prepare(Scenario scenario, out string error)
        {
            try
            {
                MovePlayer[] players = Object.FindObjectsByType<MovePlayer>(FindObjectsSortMode.None)
                    .OrderBy(p => p.playerNumber).ToArray();
                if (players.Length != 2 || players[0].playerNumber != 1 || players[1].playerNumber != 2)
                {
                    error = "プレイヤーが2体生成されていません";
                    return null;
                }

                // HPが0になってリザルト画面へ移ると、記録が途切れるため
                foreach (GameObserver observer in Object.FindObjectsByType<GameObserver>(FindObjectsSortMode.None))
                    observer.enabled = false;

                Physics2D.SyncTransforms();
                double origin = Time.fixedTimeAsDouble;
                var input1 = new ScriptedInputProvider(scenario, 1, origin);
                InputManager.Instance.inputP1 = input1;
                InputManager.Instance.inputP2 = new ScriptedInputProvider(scenario, 2, origin);

                foreach (MovePlayer p in players)
                {
                    Scenario.Init init = scenario.InitFor(p.playerNumber);
                    if (init != null && !ApplyInit(p, init, out error))
                        return null;
                }

                var recorder = new TraceRecorder(players, input1, scenario.Steps);
                InputManager.Instance.StartCoroutine(recorder.Run());
                error = null;
                return recorder;
            }
            catch (Exception e)
            {
                error = e.ToString();
                return null;
            }
        }

        // 位置は Transform から反映し（このステップの MovePlayer.FixedUpdate が transform.position を読むため）、
        // 角度と速度は剛体に直接入れる。左右反転したP2の Transform の回転を書き換えると、剛体には
        // 逆向きの角度が入り、以後ずっと当たり判定と見た目がずれるため（S7で確認）。
        // Transform の角度は、最初の物理ステップの後に剛体から書き戻される。
        private static bool ApplyInit(MovePlayer p, Scenario.Init init, out string error)
        {
            Transform t = p.transform;
            Rigidbody2D rb = p.GetComponent<Rigidbody2D>();
            Vector2 pos = init.Pos ?? (Vector2)t.position;

            t.position = new Vector3(pos.x, pos.y, t.position.z);
            Physics2D.SyncTransforms();
            if (init.AngleDeg.HasValue) rb.rotation = init.AngleDeg.Value;
            rb.linearVelocity = init.Vel ?? Vector2.zero;
            rb.angularVelocity = init.AngVelDeg ?? 0f;

            // ウルトゲージは前回の位置との差で溜まるので、瞬間移動した分が溜まらないようにそろえる
            LastPosition.SetValue(p, t.position);

            if (init.AngleDeg.HasValue && Mathf.Abs(Mathf.DeltaAngle(rb.rotation, init.AngleDeg.Value)) > 0.01f)
            {
                error = $"slot {p.playerNumber}: 角度 {init.AngleDeg} を反映できません（rb.rotation {rb.rotation}）";
                return false;
            }
            error = null;
            return true;
        }

        private static string Env(string name, string fallback)
        {
            string value = Environment.GetEnvironmentVariable(name);
            return string.IsNullOrEmpty(value) ? fallback : value;
        }
    }
}
