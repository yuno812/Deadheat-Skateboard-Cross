using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;

namespace DscSim
{
    // シナリオ1本。形式と意味は rust/crates/dsc-scenario/src/scenario.rs と同じ（Rust側が正）。
    internal sealed class Scenario
    {
        public sealed class Init
        {
            public int Slot;
            public Vector2? Pos;
            public float? AngleDeg;
            public Vector2? Vel;
            public float? AngVelDeg;
        }

        public sealed class Span
        {
            public int Slot, From, To;
            public float? MoveX, Rotate;
            public bool? Jump;
        }

        public string Id, Stage, P1, P2;
        public int Steps;
        public List<Init> Inits = new List<Init>();
        public List<Span> Inputs = new List<Span>();

        public string Character(int slot) => slot == 1 ? P1 : P2;

        public Init InitFor(int slot) => Inits.FirstOrDefault(x => x.Slot == slot);

        // 項目ごとに、そのステップを含む最後の区間で指定された値を使う
        public InputState InputAt(int slot, int step)
        {
            var input = new InputState();
            foreach (Span span in Inputs)
            {
                if (span.Slot != slot || step < span.From || step >= span.To) continue;
                if (span.MoveX.HasValue) input.moveX = span.MoveX.Value;
                if (span.Rotate.HasValue) input.rotate = span.Rotate.Value;
                if (span.Jump.HasValue) input.jumpPressed = span.Jump.Value;
            }
            return input;
        }

        public static Scenario Parse(string json)
        {
            var root = (Dictionary<string, object>)MiniJson.Parse(json);
            var scenario = new Scenario
            {
                Id = (string)root["id"],
                Stage = (string)root["stage"],
                P1 = (string)root["p1"],
                P2 = (string)root["p2"],
                Steps = Convert.ToInt32(root["steps"]),
            };
            foreach (Dictionary<string, object> o in List(root, "init"))
            {
                scenario.Inits.Add(new Init
                {
                    Slot = Convert.ToInt32(o["slot"]),
                    Pos = Vec(o, "pos"),
                    AngleDeg = Float(o, "angleDeg"),
                    Vel = Vec(o, "vel"),
                    AngVelDeg = Float(o, "angVelDeg"),
                });
            }
            foreach (Dictionary<string, object> o in List(root, "inputs"))
            {
                scenario.Inputs.Add(new Span
                {
                    Slot = Convert.ToInt32(o["slot"]),
                    From = Convert.ToInt32(o["from"]),
                    To = Convert.ToInt32(o["to"]),
                    MoveX = Float(o, "moveX"),
                    Rotate = Float(o, "rotate"),
                    Jump = o.TryGetValue("jump", out object j) ? (bool?)(bool)j : null,
                });
            }
            return scenario;
        }

        private static IEnumerable<object> List(Dictionary<string, object> o, string key) =>
            o.TryGetValue(key, out object v) && v is List<object> list ? list : Enumerable.Empty<object>();

        private static float? Float(Dictionary<string, object> o, string key) =>
            o.TryGetValue(key, out object v) ? (float?)Convert.ToSingle(v) : null;

        private static Vector2? Vec(Dictionary<string, object> o, string key)
        {
            if (!o.TryGetValue(key, out object v)) return null;
            var list = (List<object>)v;
            return new Vector2(Convert.ToSingle(list[0]), Convert.ToSingle(list[1]));
        }
    }
}
