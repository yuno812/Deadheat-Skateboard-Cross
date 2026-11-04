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
    // 狙いが要るウルトの当たり具合を測る（計測用、5 分ほど）。CpuMatchTests とは別に、名前を指定したときだけ流す：
    //   Invoke-UnityTests 'DscSim.CpuUltMeasureTests' 'cpu-aimed'（手順は rust/docs/CPU調整ガイド.md）
    [Explicit("計測用（5 分ほど）。名前を指定して流す")]
    public class CpuUltMeasureTests
    {
        private const string Stage = "R2500";

        // 狙いが要るウルト（zero の弾・nagisa の突進）が、ランダムに動く相手にどれだけ当たるかを測る（計測用）。
        // ウルトを使わない CPU（CpuController.useUltimate = false）とも比べる。
        // 各キャラ・種（Seeds 個）ごとに 60 秒。機会を増やすため、HP が 1 を切ったら元に戻して戦い続けさせる（勝ち負けは数えない）。
        // 発動から一定時間（zero は止まる 0.5 秒＋弾の時間で 1.3 秒、nagisa は 0.6 秒）以内に相手の HP が減れば当たり、
        // 自分の HP が減れば自滅とみなす（zero は弾の命中ログでも数える）
        [UnityTest, Timeout(60 * 60 * 1000)]
        public IEnumerator MeasureAimedUltimates()
        {
            yield return new EnterPlayMode();
            LogAssert.ignoreFailingMessages = true;

            Dictionary<string, SimProject.Character> characters = SimProject.SelectableCharacters().ToDictionary(c => c.Name);
            SimProject.EnsureManagers();
            // CPU の判断そのものを見るので、ハンデの無い「つよい」で流す
            InputManager.Instance.cpuDifficulty = Difficulty.Strong;
            float captureDeltaTime = Time.captureDeltaTime;
            Application.logMessageReceived += RecordUlt;
            try
            {
                foreach ((string cpu, bool useUlt) in new[] { ("zero", true), ("zero", false), ("nagisa", true), ("nagisa", false) })
                {
                    int fires = 0, hits = 0, selfHurt = 0, bulletHits = 0;
                    float dealt = 0f, taken = 0f;
                    double window = cpu == "zero" ? 1.3 : 0.6;
                    for (int seed = 1; seed <= Seeds; seed++)
                    {
                        InputManager.Instance.inputP1 = new RandomInputProvider(seed);
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
                        players[1].GetComponent<CpuController>().useUltimate = useUlt;
                        ultTimes.Clear();
                        bulletHitCount = 0;
                        float oppHp = players[0].HP, myHp = players[1].HP;
                        float startOpp = oppHp, startMe = myHp;
                        Rigidbody2D meBody = players[1].GetComponent<Rigidbody2D>(), oppBody = players[0].GetComponent<Rigidbody2D>();
                        double start = Time.fixedTimeAsDouble;
                        int counted = 0;
                        bool hitThisUlt = false, hurtThisUlt = false, reported = true;
                        while ((Time.fixedTimeAsDouble - start) < 60.0)
                        {
                            yield return null;
                            double now = Time.fixedTimeAsDouble;
                            if (ultTimes.Count > counted)
                            {
                                counted = ultTimes.Count;
                                hitThisUlt = hurtThisUlt = reported = false;
                                Debug.Log($"[DscSim] {cpu} ult #{counted}: me pos=({meBody.position.x:F2},{meBody.position.y:F2}) angle={Vector2.SignedAngle(Vector2.up, players[1].transform.up):F0} ground={players[1].GroundWheelCount}" +
                                          $" | opp pos=({oppBody.position.x:F2},{oppBody.position.y:F2}) vel=({oppBody.linearVelocity.x:F2},{oppBody.linearVelocity.y:F2}) ground={players[0].GroundWheelCount}");
                            }
                            bool inWindow = counted > 0 && now - ultTimes[counted - 1] <= window;
                            if (inWindow && players[0].HP < oppHp && !hitThisUlt) { hits++; hitThisUlt = true; }
                            if (inWindow && players[1].HP < myHp && !hurtThisUlt) { selfHurt++; hurtThisUlt = true; }
                            if (counted > 0 && !inWindow && !reported)
                            {
                                reported = true;
                                Debug.Log($"[DscSim] {cpu} ult #{counted}: {(hitThisUlt ? "hit" : "miss")}{(hurtThisUlt ? ", self hurt" : "")}");
                            }
                            dealt += Mathf.Max(0f, oppHp - players[0].HP);
                            taken += Mathf.Max(0f, myHp - players[1].HP);
                            // 機会を増やすため、HP が 1 を切ったら戻す
                            if (players[0].HP < 1f) players[0].HP = startOpp;
                            if (players[1].HP < 1f) players[1].HP = startMe;
                            oppHp = players[0].HP;
                            myHp = players[1].HP;
                        }
                        fires += ultTimes.Count;
                        bulletHits += bulletHitCount;
                    }
                    Debug.Log($"[DscSim] aimed ult {cpu} CPU (ult {(useUlt ? "on" : "off")}) vs random zero x{Seeds}: fired {fires}, hit within {window}s {hits}" +
                              (cpu == "zero" ? $" (bullet hit logs {bulletHits})" : "") +
                              $", self hurt within the window {selfHurt}, dealt {dealt} taken {taken} in {Seeds * 60}s");
                }
            }
            finally
            {
                Application.logMessageReceived -= RecordUlt;
                Time.captureDeltaTime = captureDeltaTime;
            }
            yield return new ExitPlayMode();
        }

        // 計測の種の数（1 種 60 秒）
        private const int Seeds = 20;

        private static readonly List<double> ultTimes = new List<double>();
        private static int bulletHitCount;

        private static void RecordUlt(string message, string stack, LogType type)
        {
            if (message.StartsWith("2P: ウルト")) ultTimes.Add(Time.fixedTimeAsDouble);
            // DirectionBullet（zero の弾）が P1 に当たったとき
            if (message.Contains("に命中！ ダメージ: 2")) bulletHitCount++;
        }
    }
}
