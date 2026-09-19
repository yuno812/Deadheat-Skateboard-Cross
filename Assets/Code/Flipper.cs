using UnityEngine;

public class Flipper : MonoBehaviour
{
    [Header("Player")]
    [SerializeField] private bool isPlayer1 = true;

    [Header("Direction")]
    [SerializeField] private bool reverseDirection = false;

    [Header("Hinge")]
    [SerializeField] private HingeJoint2D hinge;

    [Header("Motor")]
    [SerializeField] private float flipSpeed = 1000f;
    [SerializeField] private float returnSpeed = -1000f;
    [SerializeField] private float maxMotorTorque = 10000f;

    void FixedUpdate()
    {
        InputState input;

        if (isPlayer1)
            input = InputManager.Instance.inputP1.GetInput();
        else
            input = InputManager.Instance.inputP2.GetInput();

        float flip = flipSpeed;
        float returnValue = returnSpeed;

        if (reverseDirection)
        {
            flip *= -1f;
            returnValue *= -1f;
        }

        JointMotor2D motor = hinge.motor;
        motor.maxMotorTorque = maxMotorTorque;

        if (input.jumpPressed)
        {
            motor.motorSpeed = flip;
        }
        else
        {
            motor.motorSpeed = returnValue;
        }

        hinge.motor = motor;
        hinge.useMotor = true;
    }
}