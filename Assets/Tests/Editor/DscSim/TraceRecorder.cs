using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.Reflection;
using System.Text;
using UnityEngine;

namespace DscSim
{
    // 毎ステップ、物理計算と衝突コールバックが終わった後（WaitForFixedUpdate）の状態をCSVに記録する。
    // 形式は rust/crates/dsc-scenario/src/trace.rs と同じ。
    // Editorフォルダのスクリプトはゲームオブジェクトに付けられないため、Run() は常駐している
    // ゲーム側のオブジェクト（InputManager）のコルーチンとして動かす。
    internal sealed class TraceRecorder
    {
        public const string Header = "step,slot,x,y,angle,rot,vx,vy,w,ground,pose,hp,ult";

        private static readonly FieldInfo GroundWheelCount =
            typeof(MovePlayer).GetField("groundWheelCount", BindingFlags.Instance | BindingFlags.NonPublic);

        private sealed class Target
        {
            public MovePlayer Player;
            public Rigidbody2D Body;
            public GameObject Charge, Ult;
        }

        private readonly List<Target> targets = new List<Target>();
        private readonly StringBuilder csv = new StringBuilder();
        private readonly ScriptedInputProvider clock;
        private readonly int steps;

        public bool Done { get; private set; }
        public string Error { get; private set; }
        public string Csv => csv.ToString();

        public TraceRecorder(IEnumerable<MovePlayer> players, ScriptedInputProvider clock, int steps)
        {
            foreach (MovePlayer p in players)
            {
                targets.Add(new Target
                {
                    Player = p,
                    Body = p.GetComponent<Rigidbody2D>(),
                    Charge = p.transform.Find("Charge")?.gameObject,
                    Ult = p.transform.Find("Ult")?.gameObject,
                });
            }
            this.clock = clock;
            this.steps = steps;
            csv.Append(Header).Append('\n');
        }

        public IEnumerator Run()
        {
            for (int step = 0; step < steps; step++)
            {
                yield return new WaitForFixedUpdate();
                if (clock.CurrentStep != step)
                {
                    Error = $"step {step}: 入力側のステップ番号が {clock.CurrentStep} にずれています";
                    yield break;
                }
                foreach (Target t in targets)
                    AppendRow(step, t);
            }
            Time.timeScale = 0f;
            Done = true;
        }

        private void AppendRow(int step, Target t)
        {
            MovePlayer p = t.Player;
            Rigidbody2D rb = t.Body;
            string pose = t.Ult != null && t.Ult.activeSelf ? "Ult"
                : t.Charge != null && t.Charge.activeSelf ? "Charge"
                : "Normal";
            csv.Append(step).Append(',')
                .Append(p.playerNumber).Append(',')
                .Append(Num(rb.position.x)).Append(',')
                .Append(Num(rb.position.y)).Append(',')
                .Append(Num(Vector2.SignedAngle(Vector2.up, p.transform.up))).Append(',')
                .Append(Num(Mathf.DeltaAngle(0f, rb.rotation))).Append(',')
                .Append(Num(rb.linearVelocity.x)).Append(',')
                .Append(Num(rb.linearVelocity.y)).Append(',')
                .Append(Num(rb.angularVelocity)).Append(',')
                .Append((int)GroundWheelCount.GetValue(p)).Append(',')
                .Append(pose).Append(',')
                .Append(Num(p.HP)).Append(',')
                .Append(Num(p.ultGauge)).Append('\n');
        }

        private static string Num(float v) => v.ToString("R", CultureInfo.InvariantCulture);
    }
}
