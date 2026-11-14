using UnityEngine;

namespace DscSim
{
    // ランダムに動く相手（シミュレーターの E5 の「ランダム」と同じ動き）。10〜50 ステップごとに、
    // 前後・回転を -1/0/1、ジャンプの押しっぱなしを 30% で選び直す。
    // GetInput は FixedUpdate と Update の両方から何度も呼ばれるため、物理ステップの番号で選び直す
    internal sealed class RandomInputProvider : IInputProvider
    {
        private readonly System.Random rng;
        private InputState current;
        private long nextChange = long.MinValue;

        public RandomInputProvider(int seed)
        {
            rng = new System.Random(seed);
        }

        public InputState GetInput()
        {
            long step = (long)System.Math.Round(Time.fixedTimeAsDouble / Time.fixedDeltaTime);
            if (step >= nextChange)
            {
                nextChange = step + 10 + rng.Next(41);
                current = new InputState
                {
                    moveX = rng.Next(3) - 1,
                    rotate = rng.Next(3) - 1,
                    jumpPressed = rng.Next(10) < 3,
                };
            }
            return current;
        }
    }
}
