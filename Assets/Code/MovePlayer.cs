using UnityEngine;
using UnityEngine.InputSystem;

[RequireComponent(typeof(Rigidbody2D), typeof(SpriteRenderer))]
public class MovePlayer : MonoBehaviour
{
    [Header("Player Info")]
    [SerializeField] private PlayerState playerState;  // ← ScriptableObjectから能力値を読み込む
    public int playerNumber = 1;     // 1P or 2Pを指定（生成時に設定）

    [Header("State")]
    public float HP;
    public float attack;
    [SerializeField] private float ultCharge;

    [Header("Move")]
    [SerializeField] private float maxSpeed;
    [SerializeField] private float moveForce;
    [SerializeField] private float deceleration;

    [Header("rotation")]
    [SerializeField] private float rotationTorque;

    [Header("jump")]
    [SerializeField] private float jumpForce;
    [SerializeField] private Sprite normalSprite;
    [SerializeField] private Sprite chargingSprite;
    [SerializeField] private GameObject Normal;
    [SerializeField] private GameObject Charge;

    private Rigidbody2D rb;
    private SpriteRenderer spriteRenderer;
    private int groundWheelCount = 0;
    private bool isChargingJump = false;

    void Awake()
    {
        // コンポーネントの取得とキャラの状態の初期設定
        rb = GetComponent<Rigidbody2D>();
        spriteRenderer = GetComponent<SpriteRenderer>();
        spriteRenderer.sprite = normalSprite;

        // ScriptableObjectからパラメータを読み込み
        if (playerState != null)
        {
            rb.mass = playerState.weight; // ← 重さを設定
            maxSpeed = playerState.maxSpeed;
            moveForce = playerState.moveForce;
            deceleration = playerState.moveForce / 10; // 固定値5でもいいかも
            jumpForce = playerState.jumpForce;
            rotationTorque = playerState.rotationTorque;
            attack = playerState.attack;
            HP = playerState.hp;
            ultCharge = playerState.ultCharge;
            
        }

        rb.angularDamping = 0f; // ← 回転減衰を無効化
        rb.freezeRotation = false; // ← Z回転を物理的に固定しない
        Normal.SetActive(true);
        Charge.SetActive(false);
    }

    void FixedUpdate()
    {
        if (playerNumber == 2)
        {
            spriteRenderer.flipX = true; // 2P用に向きを反転
        }
        var keyboard = Keyboard.current;

        // --------------------------
        // プレイヤーごとに入力を分ける
        // --------------------------
        float horizontalInput = 0f;
        float rotationInput = 0f;
        bool jumpKeyPressed = false;

        if (playerNumber == 1)
        {
            // 1P操作（WASD + Space）
            if (keyboard.wKey.isPressed) horizontalInput += 1;
            if (keyboard.sKey.isPressed) horizontalInput -= 1;
            if (keyboard.aKey.isPressed) rotationInput += 1;
            if (keyboard.dKey.isPressed) rotationInput -= 1;
            jumpKeyPressed = keyboard.spaceKey.isPressed;
        }
        else if (playerNumber == 2)
        {
            // 2P操作（↑↓←→ + Enter）
            if (keyboard.upArrowKey.isPressed) horizontalInput -= 1;
            if (keyboard.downArrowKey.isPressed) horizontalInput += 1;
            if (keyboard.leftArrowKey.isPressed) rotationInput += 1;
            if (keyboard.rightArrowKey.isPressed) rotationInput -= 1;
            jumpKeyPressed = keyboard.enterKey.isPressed;
        }

        // --------------------------
        // 地上挙動（x方向のみ制御）
        // --------------------------
        Vector2 velocity = rb.linearVelocity;

        if (groundWheelCount > 0)
        {
            if (Mathf.Abs(horizontalInput) > 0)
            {
                // 加速
                velocity.x += horizontalInput * moveForce * Time.fixedDeltaTime;
            }
            else
            {
                // 減速
                velocity.x = Mathf.MoveTowards(velocity.x, 0f, deceleration * Time.fixedDeltaTime);
            }
        }

        // 最大速度制限
        velocity.x = Mathf.Clamp(velocity.x, -maxSpeed, maxSpeed);
        rb.linearVelocity = new Vector2(velocity.x, rb.linearVelocity.y);

        // --------------------------
        // 回転（空中でも地上でも可）
        // --------------------------
        if (rotationInput != 0f)
            rb.AddTorque(rotationInput * rotationTorque, ForceMode2D.Force);

        // --------------------------
        // ジャンプ処理
        // --------------------------
        if (jumpKeyPressed && !isChargingJump)
        {
            isChargingJump = true;
            spriteRenderer.sprite = chargingSprite;
            Normal.SetActive(false);
            Charge.SetActive(true);
        }

        if (isChargingJump && !jumpKeyPressed)
        {
            isChargingJump = false;
            spriteRenderer.sprite = normalSprite;

            if (groundWheelCount > 0)
            {
                // キャラの頭の方向にジャンプ
                rb.linearVelocity = rb.linearVelocity + (Vector2)(transform.up * jumpForce);
            }

            Normal.SetActive(true);
            Charge.SetActive(false);
        }
    }

    // --------------------------
    // 衝突判定の実装
    // --------------------------
    public void HitSame(GameObject hitObject)
    {
        Vector2 dir = (transform.position - hitObject.transform.parent.parent.position).normalized;
        rb.linearVelocity = dir * 10f;
    }

    public void HitAttack(GameObject hitObject, bool hitcheck)
    {
        Vector2 dir = (transform.position - hitObject.transform.parent.parent.position).normalized;
        rb.linearVelocity = dir * 25f;
        if (hitcheck)
        {
            MovePlayer enemy = hitObject.transform.parent.parent.GetComponent<MovePlayer>();
            HP -= enemy.attack;
        }
    }

    // --------------------------
    // 接地タイヤのカウント増減
    // --------------------------
    public void WheelLanded() => groundWheelCount++;
    public void WheelLifted() => groundWheelCount = Mathf.Max(groundWheelCount - 1, 0);
}
