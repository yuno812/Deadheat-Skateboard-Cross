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
    private float maxRotationSpeed = 900f;

    [Header("jump")]
    [SerializeField] private float jumpForce;
    private float areaJumpMultiplier = 1f;
    [SerializeField] private Sprite normalSprite;
    [SerializeField] private Sprite chargingSprite;
    [SerializeField] private Sprite ultSprite;
    [SerializeField] private GameObject Normal;
    [SerializeField] private GameObject Charge;
    [SerializeField] private GameObject Ult;

    private Rigidbody2D rb;
    private SpriteRenderer spriteRenderer;
    private int groundWheelCount = 0;
    private bool isChargingJump = false;
    private bool P1;
    public Vector3 spawnArea;
    public Vector3 outofArea;
    private bool ultInputBuffered = false; // 入力を一時保存する変数
    private UltimateAbility ultimateSystem;
    private bool chargebool = false;
    public bool ultimatebool = false;
    private bool normalbool = true;
    [SerializeField] private float moveDebuffMultiplier = 1f; // デバフ用の倍率
    [SerializeField] private float rotateDebuffMultiplier = 1f; // デバフ用の倍率
    [SerializeField] private float jumpDebuffMultiplier = 1f; // デバフ用の倍率

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
        if (playerNumber == 2)
        {
            transform.eulerAngles = new Vector3(transform.eulerAngles.x, 180f, transform.eulerAngles.z);
        }
    }

    void FixedUpdate()
    {
        OutofArea();
        
        // ゲージ蓄積計算を追加
        AccumulateUltGauge();

        InputState input = playerNumber == 1 ? InputManager.Instance.inputP1.GetInput() : InputManager.Instance.inputP2.GetInput();
        P1 = playerNumber == 1 ? true : false;

        if(ultimatebool)
        {
            Normal.SetActive(false);
            Charge.SetActive(false);
            Ult.SetActive(true);
            spriteRenderer.sprite = ultSprite;
        }
        else if(chargebool)
        {
            Normal.SetActive(false);
            Charge.SetActive(true);
            Ult.SetActive(false);
            spriteRenderer.sprite = chargingSprite;
        }
        else if(normalbool)
        {
            Normal.SetActive(true);
            Charge.SetActive(false);
            Ult.SetActive(false);
            spriteRenderer.sprite = normalSprite;
        }

        if (HP > 0)
        {
            HandleMove(input);
            HandleJump(input);
            HandleSpecialActions(input); // ウルト/スキル判定用（後述）
        }
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

    // 移動速度デバフ
    public void SetMoveDebuff(float multiplier)
    {
        moveDebuffMultiplier = multiplier;
    }

    // 回転速度デバフ
    public void SetRotateDebuff(float multiplier)
    {
        rotateDebuffMultiplier = multiplier;
    }

    // ジャンプ力デバフ
    public void SetJumpDebuff(float multiplier)
    {
        jumpDebuffMultiplier = multiplier;
    }

    // デバフ解除
    public void ClearDebuffs()
    {
        moveDebuffMultiplier = 1f;
        rotateDebuffMultiplier = 1f;
        jumpDebuffMultiplier = 1f;
    }

    // ウルトゲージ蓄積ロジック
    void AccumulateUltGauge()
    {
        if (opponent == null) return;

        // ★ここで判定：ウルトが「活動中」ならゲージ加算をスキップ
        if (ultimateSystem != null && ultimateSystem.IsActive())
        {
            lastPosition = transform.position; // 移動履歴だけ更新して抜ける
            return;
        }
        
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

    // 必殺技・スキルの入力処理
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

        velocity.x = Mathf.Clamp(velocity.x, -maxSpeed * moveDebuffMultiplier, maxSpeed * moveDebuffMultiplier);
        rb.linearVelocity = new Vector2(velocity.x, rb.linearVelocity.y);

        if (input.rotate != 0f)
            rb.AddTorque(input.rotate * rotationTorque * rotateDebuffMultiplier / Mathf.Abs(input.rotate), ForceMode2D.Force);
        
        rb.angularVelocity = Mathf.Clamp(rb.angularVelocity, -maxRotationSpeed * rotateDebuffMultiplier , maxRotationSpeed * rotateDebuffMultiplier);
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
            chargebool = true;
        }

        if (!input.jumpPressed && isChargingJump)
        {
            isChargingJump = false;

            if (groundWheelCount > 0)
                rb.linearVelocity += (Vector2)(transform.up * jumpForce * areaJumpMultiplier * jumpDebuffMultiplier);

            chargebool = false;
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
    // CPU（CpuController）が接地しているかを読むため
    public int GroundWheelCount => groundWheelCount;

    public void OutofArea()
    {
        if (Mathf.Abs(transform.position.x) >= Mathf.Abs(outofArea.x) || Mathf.Abs(transform.position.y) >= Mathf.Abs(outofArea.y))
        {
            HP -= 1f;
            transform.position = spawnArea;
            rb.linearVelocity = Vector2.zero;
        }
    }
}