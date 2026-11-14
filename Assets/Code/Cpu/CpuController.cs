using System.Collections.Generic;
using Dsc.Ai.Native;
using UnityEngine;
using AiInput = Dsc.Ai.Native.Input;

// CPU のキャラに付ける。物理ステップごとに1回、状況を dsc_ai_native.dll の CPU に渡し、
// 返ってきた入力を CpuInputProvider に入れる。MovePlayer より先に動くので、同じステップで反映される。
// 判断ロジックは rust/crates/dsc-ai（brain.rs）、調整済みの値は DLL に埋め込み。
[DefaultExecutionOrder(-100)]
[RequireComponent(typeof(MovePlayer))]
public class CpuController : MonoBehaviour
{
    public CpuInputProvider output;
    public Difficulty difficulty = Difficulty.Normal;
    // false にするとウルトを撃たない（使う・使わないで比べる計測用）
    public bool useUltimate = true;

    // 難易度のハンデ。試合の開始時に、difficulty の値（rust/profiles/*.toml）が入る。
    // プレイ中にインスペクターで変えると、その場で CPU に反映される（遊びながら難易度の値を決めるため）
    [Header("ハンデ（プレイ中に変えるとすぐ効く）")]
    [Tooltip("相手の状態を何ステップ遅れて見るか（1 ステップ = 0.02 秒）")]
    [Min(0)] public int reactionDelaySteps;
    [Tooltip("相手の位置をずらして見る大きさ（標準偏差。0.6 秒ごとに引き直す）")]
    [Min(0f)] public float aimNoise;
    [Tooltip("判断を入れ損ねて前のステップの操作のままにする確率")]
    [Range(0f, 1f)] public float slip;
    private (int, float, float) appliedHandicap;

    private MovePlayer me;
    private MovePlayer opponent;
    private Rigidbody2D meBody;
    private Rigidbody2D opponentBody;
    private AiCpu brain;
    private uint step;

    void Start()
    {
        me = GetComponent<MovePlayer>();
        meBody = me.GetComponent<Rigidbody2D>();
        foreach (MovePlayer p in FindObjectsByType<MovePlayer>(FindObjectsSortMode.None))
        {
            if (p != me) opponent = p;
        }
        if (opponent != null) opponentBody = opponent.GetComponent<Rigidbody2D>();

        var setup = new BrainSetup
        {
            jump_force = me.playerState != null ? me.playerState.jumpForce : 9f,
            max_speed = me.playerState != null ? me.playerState.maxSpeed : 5f,
            deceleration = me.playerState != null ? me.playerState.moveForce / 10f : 1f,
            gravity_y = Physics2D.gravity.y,
            dt = Time.fixedDeltaTime,
        };
        CollectStage(out float[] xy, out uint[] counts);
        try
        {
            brain = AiCpu.CreateBrain(CharacterName(), difficulty, (ulong)System.Environment.TickCount, setup, xy, counts);
            if (brain.TryGetHandicap(out reactionDelaySteps, out aimNoise, out slip))
            {
                appliedHandicap = (reactionDelaySteps, aimNoise, slip);
                Debug.Log($"[CpuController] {CharacterName()} {difficulty}: 反応の遅れ {reactionDelaySteps} ステップ、狙いのブレ {aimNoise}、もたつき {slip}");
            }
        }
        catch (System.Exception e)
        {
            // DLL が無い・古いときは CPU なし（何もしない）で続ける
            Debug.LogError($"[CpuController] {e.Message}");
        }
    }

    void FixedUpdate()
    {
        if (brain == null || opponent == null || output == null) return;
        var handicap = (reactionDelaySteps, aimNoise, slip);
        if (handicap != appliedHandicap && brain.SetHandicap(reactionDelaySteps, aimNoise, slip))
        {
            appliedHandicap = handicap;
            Debug.Log($"[CpuController] ハンデを変更：反応の遅れ {reactionDelaySteps} ステップ、狙いのブレ {aimNoise}、もたつき {slip}");
        }
        var obs = new Observation
        {
            step = step++,
            me = Observe(me, meBody),
            opp = Observe(opponent, opponentBody),
            gravity_up = GravityUp(),
            bounds = new Vec2 { x = Mathf.Abs(me.outofArea.x), y = Mathf.Abs(me.outofArea.y) },
        };
        AiInput input = brain.Act(obs);
        output.Current = new InputState { moveX = input.move_x, rotate = input.rotate, jumpPressed = input.jump };
        // ウルトのボタンは MovePlayer が Update で受け付けるため、入力で渡すと物理ステップとずれて取りこぼしたり
        // 2回撃ったりする。このステップの判定に直接入れる（ゲージが満タンでなければ何も起きない）
        if (input.ult && useUltimate) me.RequestUltimate();
    }

    void OnDestroy()
    {
        brain?.Dispose();
        brain = null;
        if (output != null) output.Current = new InputState();
    }

    // 自分にかかる重力の逆向き。重力反転のステージ（StageGravity）では gravityScale が -1 になり、上下が入れ替わる。
    // 重力が切れているあいだ（gravityScale 0：nagisa・zero のウルト中）は、前の向きのまま
    private Vector2 gravityUp = Vector2.up;

    private Vec2 GravityUp()
    {
        Vector2 g = Physics2D.gravity * meBody.gravityScale;
        if (g.sqrMagnitude > 1e-6f) gravityUp = -g.normalized;
        return new Vec2 { x = gravityUp.x, y = gravityUp.y };
    }

    private static BodyObs Observe(MovePlayer p, Rigidbody2D rb)
    {
        return new BodyObs
        {
            pos = new Vec2 { x = rb.position.x, y = rb.position.y },
            vel = new Vec2 { x = rb.linearVelocity.x, y = rb.linearVelocity.y },
            angle_deg = Vector2.SignedAngle(Vector2.up, p.transform.up),
            ang_vel_deg = rb.angularVelocity,
            ground_count = (byte)Mathf.Clamp(p.GroundWheelCount, 0, 255),
            hp = p.HP,
            ult = p.ultGauge,
            ult_max = p.ultAmount,
            facing = p.playerNumber == 1 ? 1f : -1f,
        };
    }

    // Prefab のファイル名の小文字（"zero(Clone)" → "zero"）。調整済みの値はこの名前で引く
    private string CharacterName()
    {
        string name = gameObject.name;
        int clone = name.IndexOf("(Clone)", System.StringComparison.Ordinal);
        if (clone >= 0) name = name.Substring(0, clone);
        return name.Trim().ToLowerInvariant();
    }

    // ステージの当たり判定（プレイヤー以外）を、物理エンジンが使っている凸多角形のままワールド座標で集める
    private static void CollectStage(out float[] xy, out uint[] counts)
    {
        var points = new List<float>();
        var sizes = new List<uint>();
        var group = new PhysicsShapeGroup2D();
        var vertices = new List<Vector2>();
        foreach (Collider2D col in FindObjectsByType<Collider2D>(FindObjectsSortMode.None))
        {
            if (!col.enabled || !col.gameObject.activeInHierarchy || col.isTrigger) continue;
            if (col.GetComponentInParent<MovePlayer>() != null) continue;
            int n = col.GetShapes(group);
            Matrix4x4 toWorld = group.localToWorldMatrix;
            for (int i = 0; i < n; i++)
            {
                if (group.GetShape(i).shapeType != PhysicsShapeType2D.Polygon) continue;
                group.GetShapeVertices(i, vertices);
                foreach (Vector2 v in vertices)
                {
                    Vector2 w = toWorld.MultiplyPoint3x4(v);
                    points.Add(w.x);
                    points.Add(w.y);
                }
                sizes.Add((uint)vertices.Count);
            }
        }
        xy = points.ToArray();
        counts = sizes.ToArray();
    }
}
