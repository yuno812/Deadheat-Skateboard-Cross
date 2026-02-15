using UnityEngine;
using UnityEngine.InputSystem;

[RequireComponent(typeof(Rigidbody2D), typeof(SpriteRenderer))]
public class MovePlayer : MonoBehaviour
{
    [Header("Player Info")]
    public PlayerState playerState;
    public int playerNumber = 1;

    [Header("State")]
    public float HP;
    public float attack;
    public float ultAmount;
    public float ultGauge;

    [Header("Move")]
    [SerializeField] private float maxSpeed;
    [SerializeField] private float moveForce;
    [SerializeField] private float deceleration;

    // --- ウルトゲージ用に追加 ---
    [Header("Ultimate Gauge settings")]
    [SerializeField] private float ultGainMultiplier = 5f; // 溜まりやすさ（調整用）
    private MovePlayer opponent;
    private Vector3 lastPosition;
    // --------------------------

    [Header("rotation")]
    [SerializeField] private float rotationTorque;

    [Header("jump")]
    [SerializeField] private float jumpForce;
    private float areaJumpMultiplier = 1f;
    [SerializeField] private Sprite normalSprite;
    [SerializeField] private Sprite chargingSprite;
    [SerializeField] private GameObject Normal;
    [SerializeField] private GameObject Charge;

    private Rigidbody2D rb;
    private SpriteRenderer spriteRenderer;
    private int groundWheelCount = 0;
    private bool isChargingJump = false;
    private bool P1;
    public Vector3 spawnArea;
    public Vector3 outofArea;
    private bool ultInputBuffered = false; // 入力を一時保存する変数
    private UltimateAbility ultimateSystem;

    void Awake()
    {
        rb = GetComponent<Rigidbody2D>();
        spriteRenderer = GetComponent<SpriteRenderer>();
        spriteRenderer.sprite = normalSprite;
        ultimateSystem = GetComponent<UltimateAbility>();

        if (playerState != null)
        {
            rb.mass = playerState.weight;
            maxSpeed = playerState.maxSpeed;
            moveForce = playerState.moveForce;
            deceleration = playerState.moveForce / 10;
            jumpForce = playerState.jumpForce;
            rotationTorque = playerState.rotationTorque;
            attack = playerState.attack;
            HP = playerState.hp;
            ultAmount = playerState.ultAmount;
        }

        rb.angularDamping = 0f;
        rb.freezeRotation = false;
        Normal.SetActive(true);
        Charge.SetActive(false);
    }

    void Start()
    {
        lastPosition = transform.position;

        // 相手プレイヤー（自分とは違う番号）を自動的に探す
        MovePlayer[] allPlayers = FindObjectsByType<MovePlayer>(FindObjectsSortMode.None);
        foreach (var p in allPlayers)
        {
            if (p.playerNumber != this.playerNumber)
            {
                opponent = p;
                break;
            }
        }
    }

    void FixedUpdate()
    {
        OutofArea();
        
        // ゲージ蓄積計算を追加
        AccumulateUltGauge();

        if (playerNumber == 2)
        {
            spriteRenderer.flipX = true;
        }

        InputState input = playerNumber == 1 ? InputManager.Instance.inputP1.GetInput() : InputManager.Instance.inputP2.GetInput();
        P1 = playerNumber == 1 ? true : false;

        HandleMove(input);
        HandleJump(input);
        HandleSpecialActions(input); // ウルト/スキル判定用（後述）
    }

    void Update()
    {
        InputState input = playerNumber == 1 ? InputManager.Instance.inputP1.GetInput() : InputManager.Instance.inputP2.GetInput();

        // 押された瞬間を記録しておく
        if (input.ultimatePressed)
        {
            ultInputBuffered = true;
        }
    }

    // ★新規追加：ウルトゲージ蓄積ロジック
    void AccumulateUltGauge()
    {
        if (opponent == null) return;

        // 1. このフレームでの移動ベクトル
        Vector3 movement = transform.position - lastPosition;

        // 2. 相手への方向ベクトル
        Vector3 directionToOpponent = (opponent.transform.position - transform.position).normalized;

        // 3. 内積（Dot Product）で「相手に向かう方向」への移動距離を計算
        float approachDistance = Vector3.Dot(movement, directionToOpponent);

        // 4. 近づいている（プラス）時だけ、倍率をかけてゲージ加算
        if (approachDistance > 0)
        {
            ultGauge += approachDistance * ultGainMultiplier;
            ultGauge = Mathf.Clamp(ultGauge, 0f, ultAmount);
        }

        // 位置の更新
        lastPosition = transform.position;
    }

    // ★新規追加：必殺技・スキルの入力処理
    void HandleSpecialActions(InputState input)
    {
        // ウルトボタン（：キー や パッド△）
        if (ultInputBuffered && ultGauge >= (ultAmount - 0.1f))
        {
            // アタッチされているウルトスクリプトがあるか確認
            if (ultimateSystem != null)
            {
                ultGauge = 0f;
                ultInputBuffered = false; // 実行したのでリセット
                
                ultimateSystem.Execute(this); 
                
                Debug.Log($"{playerNumber}P: ウルト「{ultimateSystem.GetType().Name}」発動！");
            }
        }

        // 押されたけどゲージが足りなかった場合も、
        // 次のフレームでまた押す必要があるならリセットしておく
        if (ultInputBuffered)
        {
            ultInputBuffered = false; 
        }
        // スキルボタン（」キー や パッド□）
        if (input.skillPressed)
        {
            // クールタイム判定などを入れてスキル実行
        }
    }

    void HandleMove(InputState input)
    {
        Vector2 velocity = rb.linearVelocity;
        // P1/P2の向き制御（既存ロジック）
        float moveDir = P1 ? input.moveX : -input.moveX;

        if (groundWheelCount > 0)
        {
            if (Mathf.Abs(moveDir) > 0)
                velocity.x += moveDir * moveForce * Time.fixedDeltaTime;
            else
                velocity.x = Mathf.MoveTowards(velocity.x, 0f, deceleration * Time.fixedDeltaTime);
        }

        velocity.x = Mathf.Clamp(velocity.x, -maxSpeed, maxSpeed);
        rb.linearVelocity = new Vector2(velocity.x, rb.linearVelocity.y);

        if (input.rotate != 0f)
            rb.AddTorque(input.rotate * rotationTorque, ForceMode2D.Force);
    }

    public void SetJumpMultiplier(float multiplier)
    {
        areaJumpMultiplier = multiplier;
    }

    void HandleJump(InputState input)
    {
        if (input.jumpPressed && !isChargingJump)
        {
            isChargingJump = true;
            spriteRenderer.sprite = chargingSprite;
            Normal.SetActive(false);
            Charge.SetActive(true);
        }

        if (!input.jumpPressed && isChargingJump)
        {
            isChargingJump = false;
            spriteRenderer.sprite = normalSprite;

            if (groundWheelCount > 0)
                rb.linearVelocity += (Vector2)(transform.up * jumpForce * areaJumpMultiplier);

            Normal.SetActive(true);
            Charge.SetActive(false);
        }
    }

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

    public void WheelLanded() => groundWheelCount++;
    public void WheelLifted() => groundWheelCount = Mathf.Max(groundWheelCount - 1, 0);

    public void OutofArea()
    {
        if (Mathf.Abs(transform.position.x) >= Mathf.Abs(outofArea.x) || Mathf.Abs(transform.position.y) >= Mathf.Abs(outofArea.y))
        {
            transform.position = spawnArea;
        }
    }
}