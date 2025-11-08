using UnityEngine;
using UnityEngine.InputSystem;

[RequireComponent(typeof(Rigidbody2D), typeof(SpriteRenderer))]
public class MovePlayer : MonoBehaviour
{
    [Header("Move")]
    [SerializeField] private float maxSpeed = 5f;
    [SerializeField] private float moveForce = 50f;
    [SerializeField] private float deceleration = 5f;
    [Header("rotation")]
    [SerializeField] private float rotationTorque = 200f;
    [Header("jump")]
    [SerializeField] private float jumpForce = 10f;
    [SerializeField] private Sprite normalSprite;
    [SerializeField] private Sprite chargingSprite;
    [SerializeField] private GameObject Normal;
    [SerializeField] private GameObject Charge;

    private Rigidbody2D rb;
    private SpriteRenderer spriteRenderer;
    private int groundWheelCount = 0;
    private bool isChargingJump = false;

    void Start()
    {
        rb = GetComponent<Rigidbody2D>();
        spriteRenderer = GetComponent<SpriteRenderer>();
        spriteRenderer.sprite = normalSprite;
        rb.angularDamping = 0.1f;
        Normal.SetActive(true);
        Charge.SetActive(false);
    }

    void FixedUpdate()
    {
        var keyboard = Keyboard.current;

        // 入力取得
        float horizontalInput = 0f;
        if (keyboard.wKey.isPressed || keyboard.upArrowKey.isPressed) horizontalInput += 1;
        if (keyboard.sKey.isPressed || keyboard.downArrowKey.isPressed) horizontalInput -= 1;

        float rotationInput = 0f;
        if (keyboard.aKey.isPressed || keyboard.leftArrowKey.isPressed) rotationInput += 1;
        if (keyboard.dKey.isPressed || keyboard.rightArrowKey.isPressed) rotationInput -= 1;

        // --------------------------
        // 地上挙動（x方向のみ制御）
        // --------------------------
        if (groundWheelCount > 0)
        {
            if (Mathf.Abs(horizontalInput) > 0)
                rb.AddForce(new Vector2(horizontalInput * moveForce, 0f), ForceMode2D.Force);
            else
                rb.linearVelocity = new Vector2(
                    Mathf.MoveTowards(rb.linearVelocity.x, 0f, deceleration * Time.fixedDeltaTime),
                    rb.linearVelocity.y
                );
        }

        // x方向速度制限（y方向は変更しない）
        float clampedX = Mathf.Clamp(rb.linearVelocity.x, -maxSpeed, maxSpeed);
        rb.linearVelocity = new Vector2(clampedX, rb.linearVelocity.y);

        // --------------------------
        // 回転（空中でも地上でも可）
        // --------------------------
        if (rotationInput != 0f)
            rb.AddTorque(rotationInput * rotationTorque, ForceMode2D.Force);

        // --------------------------
        // ジャンプ処理
        // --------------------------
        bool jumpKeyPressed = keyboard.spaceKey.isPressed || keyboard.enterKey.isPressed;

        // 長押し中のスプライト切替（空中でも可）
        if (jumpKeyPressed && !isChargingJump)
        {
            isChargingJump = true;
            spriteRenderer.sprite = chargingSprite;
            Normal.SetActive(false);
            Charge.SetActive(true);
        }

        // キー離した瞬間にジャンプ（地上のみ）
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

    // 接地タイヤのカウント増減
    public void WheelLanded()
    {
        groundWheelCount++;
    }

    public void WheelLifted()
    {
        groundWheelCount = Mathf.Max(groundWheelCount - 1, 0);
    }
}
