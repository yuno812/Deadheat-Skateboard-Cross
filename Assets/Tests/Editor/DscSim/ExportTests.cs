using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Security.Cryptography;
using System.Text;
using NUnit.Framework;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;
using UnityEngine.TestTools;
using Object = UnityEngine.Object;

namespace DscSim
{
    // CPUシミュレーター用に、ステージとキャラの形・数値を rust/data/export/<ステージ>/ へ書き出す。
    // ゲームのコードやシーンは変更しない。実行は scripts/export-sim-data.ps1。
    //
    // 実際にステージを起動し、Awake/Start が終わった時点（物理は1ステップも進めない）の値を取る。
    // キャラの値は、GameManager がかける playerScale や、MovePlayer.Awake が上書きする質量を含むため。
    public class ExportTests
    {
        private const int Schema = 1;
        private static readonly string[] Stages = { "R2500" };
        private static readonly string[] Poses = { "Normal", "Charge", "Ult" };

        private static readonly string[] SettingsSources =
        {
            "ProjectSettings/Physics2DSettings.asset",
            "ProjectSettings/TimeManager.asset",
            "ProjectSettings/TagManager.asset",
        };

        [UnityTest]
        public IEnumerator Export([ValueSource(nameof(Stages))] string stage)
        {
            yield return new EnterPlayMode();

            // シーン内のUIなどがエラーを出しても、書き出しは続ける
            LogAssert.ignoreFailingMessages = true;

            string scenePath = SimProject.StageScenePath(stage);
            string outDir = Path.Combine(SimProject.DataDir(), "export", stage);
            Directory.CreateDirectory(outDir);

            List<SimProject.Character> characters = SimProject.SelectableCharacters();
            Assert.IsNotEmpty(characters, $"{SimProject.SelectScenePath} からキャラが見つかりません");
            SimProject.EnsureManagers();

            float timeScale = Time.timeScale;
            Time.timeScale = 0f;
            try
            {
                bool stageWritten = false;
                foreach (SimProject.Character character in characters)
                {
                    // 同じキャラをP1とP2に置き、反転なし・反転ありの両方を1回で取る
                    PlayerSelection.Instance.playerPrefabP1 = character.Prefab;
                    PlayerSelection.Instance.playerPrefabP2 = character.Prefab;
                    EditorSceneManager.LoadSceneInPlayMode(scenePath, new LoadSceneParameters(LoadSceneMode.Single));
                    yield return null;
                    yield return null;
                    Physics2D.SyncTransforms();

                    MovePlayer[] players = Object.FindObjectsByType<MovePlayer>(FindObjectsSortMode.None)
                        .OrderBy(p => p.playerNumber).ToArray();
                    Assert.AreEqual(2, players.Length, $"{character.Name}: プレイヤーが2体生成されていません");
                    GameManager gm = Object.FindFirstObjectByType<GameManager>();
                    Assert.IsNotNull(gm, $"{scenePath} に GameManager がありません");

                    if (!stageWritten)
                    {
                        WriteFile(Path.Combine(outDir, "stage.json"), StageJson(stage, scenePath, gm));
                        stageWritten = true;
                    }
                    WriteFile(Path.Combine(outDir, character.Name + ".json"),
                        CharacterJson(stage, scenePath, character, players));
                }
            }
            finally
            {
                Time.timeScale = timeScale;
            }

            yield return new ExitPlayMode();
        }

        // ---- ステージ ----

        private static string StageJson(string stage, string scenePath, GameManager gm)
        {
            var w = new JsonWriter();
            w.BeginObject()
                .Prop("schema", Schema)
                .Prop("unity", Application.unityVersion)
                .Prop("stage", stage)
                .Prop("scene", scenePath);

            w.Key("world");
            WriteWorld(w);

            w.Key("spawn").BeginArray()
                .Value((Vector2)gm.spawnPointP1.position)
                .Value((Vector2)gm.spawnPointP2.position)
                .EndArray();
            w.Prop("outOfArea", (Vector2)gm.outofArea.position);
            w.Prop("playerScale", (Vector2)gm.playerScale);

            var materials = new List<string>();
            w.Key("shapes").BeginArray();
            foreach (Collider2D col in Object.FindObjectsByType<Collider2D>(FindObjectsSortMode.None)
                         .Where(c => c.GetComponentInParent<MovePlayer>(true) == null && IsLive(c))
                         .OrderBy(c => HierarchyPath(c.transform, null), StringComparer.Ordinal))
            {
                foreach (ShapeRecord s in CaptureShapes(col, "Always", null))
                    WriteShape(w, s);
                materials.AddRange(MaterialPaths(col));
            }
            w.EndArray();

            w.Key("sources");
            WriteSources(w, new[] { scenePath }.Concat(SettingsSources).Concat(materials));
            w.EndObject();
            return w.ToString();
        }

        private static void WriteWorld(JsonWriter w)
        {
            w.BeginObject()
                .Prop("gravity", Physics2D.gravity)
                .Prop("dt", Time.fixedDeltaTime)
                .Prop("velocityIterations", Physics2D.velocityIterations)
                .Prop("positionIterations", Physics2D.positionIterations)
                .Prop("baumgarteScale", Physics2D.baumgarteScale)
                .Prop("baumgarteTOIScale", Physics2D.baumgarteTOIScale)
                .Prop("defaultContactOffset", Physics2D.defaultContactOffset)
                .Prop("maxLinearCorrection", Physics2D.maxLinearCorrection)
                .Prop("maxAngularCorrection", Physics2D.maxAngularCorrection)
                .Prop("maxTranslationSpeed", Physics2D.maxTranslationSpeed)
                .Prop("maxRotationSpeed", Physics2D.maxRotationSpeed)
                .Prop("bounceThreshold", Physics2D.bounceThreshold)
                .Prop("contactThreshold", Physics2D.contactThreshold)
                .Prop("linearSleepTolerance", Physics2D.linearSleepTolerance)
                .Prop("angularSleepTolerance", Physics2D.angularSleepTolerance)
                .Prop("timeToSleep", Physics2D.timeToSleep)
                .Prop("useSubStepping", Physics2D.useSubStepping)
                .Prop("useSubStepContacts", Physics2D.useSubStepContacts)
                .Prop("useMultithreading", Physics2D.jobOptions.useMultithreading)
                .Prop("callbacksOnDisable", Physics2D.callbacksOnDisable)
                .Prop("autoSyncTransforms", Physics2D.autoSyncTransforms)
                .Prop("queriesHitTriggers", Physics2D.queriesHitTriggers)
                .Prop("simulationMode", Physics2D.simulationMode.ToString());

            // layerCollisionMasks[i] のビット j が立っていれば、レイヤー i と j は衝突する
            w.Key("layerCollisionMasks").BeginArray();
            for (int i = 0; i < 32; i++)
                w.Value(Physics2D.GetLayerCollisionMask(i));
            w.EndArray();
            w.EndObject();
        }

        // ---- キャラ ----

        private static string CharacterJson(string stage, string scenePath, SimProject.Character character, MovePlayer[] players)
        {
            var w = new JsonWriter();
            w.BeginObject()
                .Prop("schema", Schema)
                .Prop("unity", Application.unityVersion)
                .Prop("stage", stage)
                .Prop("character", character.Name)
                .Prop("prefab", character.PrefabPath);

            // MovePlayer.Awake で PlayerState から反映された後の値
            MovePlayer mp = players[0];
            w.Key("stats").BeginObject()
                .Prop("maxSpeed", Field(mp, "maxSpeed"))
                .Prop("moveForce", Field(mp, "moveForce"))
                .Prop("deceleration", Field(mp, "deceleration"))
                .Prop("jumpForce", Field(mp, "jumpForce"))
                .Prop("rotationTorque", Field(mp, "rotationTorque"))
                .Prop("maxRotationSpeed", Field(mp, "maxRotationSpeed"))
                .Prop("attack", mp.attack)
                .Prop("hp", mp.HP)
                .Prop("ultAmount", mp.ultAmount)
                .Prop("ultGainMultiplier", Field(mp, "ultGainMultiplier"))
                .EndObject();

            var materials = new List<string>();
            w.Key("slots").BeginArray();
            foreach (MovePlayer p in players)
            {
                WriteSlot(w, p);
                foreach (Collider2D col in p.GetComponentsInChildren<Collider2D>(true))
                    materials.AddRange(MaterialPaths(col));
            }
            w.EndArray();

            var sources = new List<string> { character.PrefabPath, scenePath };
            if (mp.playerState != null) sources.Add(AssetDatabase.GetAssetPath(mp.playerState));
            w.Key("sources");
            WriteSources(w, sources.Concat(materials));
            w.EndObject();
            return w.ToString();
        }

        private static void WriteSlot(JsonWriter w, MovePlayer p)
        {
            Rigidbody2D rb = p.GetComponent<Rigidbody2D>();
            var poseObjects = Poses
                .Select(name => p.transform.Find(name))
                .Where(t => t != null)
                .Select(t => t.gameObject)
                .ToArray();
            bool[] wasActive = poseObjects.Select(g => g.activeSelf).ToArray();

            // 姿勢ごとに、その姿勢の子だけを有効にして形と慣性モーメントを取る
            var shapes = new List<ShapeRecord>();
            var massByPose = new List<(string pose, float inertia, Vector2 com)>();
            try
            {
                foreach (GameObject pose in poseObjects)
                {
                    foreach (GameObject g in poseObjects)
                        g.SetActive(g == pose);
                    Physics2D.SyncTransforms();
                    massByPose.Add((pose.name, rb.inertia, rb.centerOfMass));
                    foreach (Collider2D col in pose.GetComponentsInChildren<Collider2D>(false).Where(IsLive))
                        shapes.AddRange(CaptureShapes(col, pose.name, rb));
                }
            }
            finally
            {
                for (int i = 0; i < poseObjects.Length; i++)
                    poseObjects[i].SetActive(wasActive[i]);
                Physics2D.SyncTransforms();
            }

            // どの姿勢の子にも属さない当たり判定は常に有効
            foreach (Collider2D col in p.GetComponentsInChildren<Collider2D>(false)
                         .Where(c => IsLive(c) && !poseObjects.Any(g => c.transform.IsChildOf(g.transform))))
                shapes.AddRange(CaptureShapes(col, "Always", rb));

            Debug.Log($"[DscSim] {p.name} slot{p.playerNumber}: rb.rotation={rb.rotation} euler={p.transform.eulerAngles} lossyScale={p.transform.lossyScale}");
            w.BeginObject()
                .Prop("slot", p.playerNumber)
                .Prop("mirrored", Mathf.Abs(Mathf.DeltaAngle(p.transform.eulerAngles.y, 180f)) < 1f);

            w.Key("body").BeginObject()
                .Prop("mass", rb.mass)
                .Prop("useAutoMass", rb.useAutoMass)
                .Prop("gravityScale", rb.gravityScale)
                .Prop("linearDamping", rb.linearDamping)
                .Prop("angularDamping", rb.angularDamping)
                .Prop("bodyType", rb.bodyType.ToString())
                .Prop("collisionDetection", rb.collisionDetectionMode.ToString())
                .Prop("sleepMode", rb.sleepMode.ToString())
                .Prop("interpolation", rb.interpolation.ToString())
                .Prop("constraints", rb.constraints.ToString())
                .Prop("includeLayers", rb.includeLayers.value)
                .Prop("excludeLayers", rb.excludeLayers.value)
                .Prop("pos", rb.position)
                .Prop("angleDeg", rb.rotation)
                .EndObject();

            w.Key("poses").BeginObject();
            foreach (var (pose, inertia, com) in massByPose)
            {
                w.Key(pose).BeginObject()
                    .Prop("inertia", inertia)
                    .Prop("centerOfMass", com)
                    .EndObject();
            }
            w.EndObject();

            w.Key("shapes").BeginArray();
            foreach (ShapeRecord s in shapes)
                WriteShape(w, s);
            w.EndArray();
            w.EndObject();
        }

        private static float Field(MovePlayer p, string name)
        {
            FieldInfo f = typeof(MovePlayer).GetField(name, BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic);
            Assert.IsNotNull(f, $"MovePlayer.{name} が見つかりません");
            return Convert.ToSingle(f.GetValue(p));
        }

        // ---- 当たり判定の形 ----

        private sealed class ShapeRecord
        {
            public Collider2D Collider;
            public string Object;
            public string Pose;
            public string Kind;
            public float Radius;
            public List<Vector2> Vertices;
            public bool UseAdjacentStart, UseAdjacentEnd;
            public Vector2 AdjacentStart, AdjacentEnd;
        }

        // 物理エンジンが実際に使っている形（PolygonCollider2D の凸分割後など）を取り出す。
        // 頂点は、bodyを渡せばその剛体のローカル（位置と角度だけ。拡大縮小と反転は頂点に含む）、null ならワールド。
        private static List<ShapeRecord> CaptureShapes(Collider2D col, string pose, Rigidbody2D body)
        {
            var group = new PhysicsShapeGroup2D();
            int count = col.GetShapes(group);
            Matrix4x4 toWorld = group.localToWorldMatrix;
            float radiusScale = ((Vector2)toWorld.GetColumn(0)).magnitude;
            // 仕様書の未決事項「GetShapes が返す座標系」の確認用
            Matrix4x4 expected = body != null ? body.localToWorldMatrix : Matrix4x4.identity;
            if (count > 0 && !Approximately(toWorld, expected))
                Debug.Log($"[DscSim] {HierarchyPath(col.transform, null)}: shapes are not in {(body != null ? "body" : "world")} space\n{toWorld}");

            var result = new List<ShapeRecord>();
            var vertices = new List<Vector2>();
            for (int i = 0; i < count; i++)
            {
                PhysicsShape2D shape = group.GetShape(i);
                group.GetShapeVertices(i, vertices);
                result.Add(new ShapeRecord
                {
                    Collider = col,
                    Object = HierarchyPath(col.transform, body != null ? body.transform : null),
                    Pose = pose,
                    Kind = shape.shapeType.ToString(),
                    Radius = shape.radius * radiusScale,
                    Vertices = vertices.Select(v => ToBodySpace(toWorld.MultiplyPoint3x4(v), body)).ToList(),
                    UseAdjacentStart = shape.useAdjacentStart,
                    UseAdjacentEnd = shape.useAdjacentEnd,
                    AdjacentStart = ToBodySpace(toWorld.MultiplyPoint3x4(shape.adjacentStart), body),
                    AdjacentEnd = ToBodySpace(toWorld.MultiplyPoint3x4(shape.adjacentEnd), body),
                });
            }
            return result;
        }

        private static void WriteShape(JsonWriter w, ShapeRecord s)
        {
            Collider2D col = s.Collider;
            w.BeginObject()
                .Prop("object", s.Object)
                .Prop("collider", col.GetType().Name)
                .Prop("pose", s.Pose)
                .Prop("tag", col.tag)
                .Prop("layer", col.gameObject.layer);

            // 当たったときの処理はタグではなく、同じオブジェクトのスクリプト（WheelGroundCheck など）が決める
            w.Key("handlers").BeginArray();
            foreach (MonoBehaviour mb in col.GetComponents<MonoBehaviour>().Where(m => m != null))
                w.Value(mb.GetType().Name);
            w.EndArray();

            w.Prop("trigger", col.isTrigger)
                .Prop("kind", s.Kind)
                .Prop("radius", s.Radius);
            w.Key("vertices").BeginArray();
            foreach (Vector2 v in s.Vertices)
                w.Value(v);
            w.EndArray();
            if (s.Kind == nameof(PhysicsShapeType2D.Edges))
            {
                w.Prop("useAdjacentStart", s.UseAdjacentStart)
                    .Prop("useAdjacentEnd", s.UseAdjacentEnd)
                    .Prop("adjacentStart", s.AdjacentStart)
                    .Prop("adjacentEnd", s.AdjacentEnd);
            }
            w.Prop("friction", col.friction)
                .Prop("bounciness", col.bounciness)
                .Prop("frictionCombine", col.frictionCombine.ToString())
                .Prop("bounceCombine", col.bounceCombine.ToString())
                .Prop("density", col.density)
                .Prop("includeLayers", col.includeLayers.value)
                .Prop("excludeLayers", col.excludeLayers.value)
                .EndObject();
        }

        private static Vector2 ToBodySpace(Vector2 world, Rigidbody2D body)
        {
            if (body == null) return world;
            Vector2 d = world - body.position;
            float a = -body.rotation * Mathf.Deg2Rad;
            float c = Mathf.Cos(a), s = Mathf.Sin(a);
            return new Vector2(c * d.x - s * d.y, s * d.x + c * d.y);
        }

        private static bool IsLive(Collider2D c) => c.enabled && c.gameObject.activeInHierarchy;

        private static bool Approximately(Matrix4x4 a, Matrix4x4 b)
        {
            for (int i = 0; i < 16; i++)
                if (Mathf.Abs(a[i] - b[i]) > 1e-4f) return false;
            return true;
        }

        // root より下の名前を "/" でつなぐ。root が null ならシーンのルートから
        private static string HierarchyPath(Transform t, Transform root)
        {
            var names = new List<string>();
            for (Transform cur = t; cur != null && cur != root; cur = cur.parent)
                names.Add(cur.name);
            names.Reverse();
            return string.Join("/", names);
        }

        private static IEnumerable<string> MaterialPaths(Collider2D col)
        {
            foreach (PhysicsMaterial2D m in new[] { col.sharedMaterial, col.attachedRigidbody != null ? col.attachedRigidbody.sharedMaterial : null })
            {
                if (m == null) continue;
                string path = AssetDatabase.GetAssetPath(m);
                if (!string.IsNullOrEmpty(path)) yield return path;
            }
        }

        // ---- 元ファイルのハッシュ ----

        // 書き出した後に元ファイルが変わったかを、Rust側（dsc check）で見分けるために記録する。
        // 改行コードの違い（git の autocrlf）で別物扱いにならないよう、CRを除いてから計算する。
        private static void WriteSources(JsonWriter w, IEnumerable<string> paths)
        {
            w.BeginArray();
            foreach (string path in paths.Distinct().OrderBy(p => p, StringComparer.Ordinal))
            {
                w.BeginObject()
                    .Prop("path", path)
                    .Prop("sha256", Sha256WithoutCr(Path.Combine(SimProject.ProjectRoot(), path)))
                    .EndObject();
            }
            w.EndArray();
        }

        private static string Sha256WithoutCr(string file)
        {
            byte[] bytes = File.ReadAllBytes(file).Where(b => b != (byte)'\r').ToArray();
            using (SHA256 sha = SHA256.Create())
                return string.Concat(sha.ComputeHash(bytes).Select(b => b.ToString("x2")));
        }

        private static void WriteFile(string path, string json)
        {
            File.WriteAllText(path, json + "\n", new UTF8Encoding(false));
            Debug.Log($"[DscSim] wrote {path}");
        }
    }
}
