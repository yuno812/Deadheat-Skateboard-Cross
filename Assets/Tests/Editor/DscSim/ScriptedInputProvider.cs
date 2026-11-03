using UnityEngine;

namespace DscSim
{
    // シナリオどおりの入力を返す。GetInput は FixedUpdate と Update の両方から何度も呼ばれるため、
    // 呼ばれた回数ではなく、再生を始めてから何回目の物理ステップかで入力を決める。
    internal sealed class ScriptedInputProvider : IInputProvider
    {
        private readonly Scenario scenario;
        private readonly int slot;
        private readonly double originFixedTime;

        // originFixedTime：再生を始める直前（時間を止めている間）の Time.fixedTimeAsDouble
        public ScriptedInputProvider(Scenario scenario, int slot, double originFixedTime)
        {
            this.scenario = scenario;
            this.slot = slot;
            this.originFixedTime = originFixedTime;
        }

        // 最初の物理ステップの FixedUpdate 中に 0。Update 中は直前に終わったステップの番号
        public int CurrentStep =>
            (int)System.Math.Round((Time.fixedTimeAsDouble - originFixedTime) / Time.fixedDeltaTime) - 1;

        public InputState GetInput()
        {
            int step = CurrentStep;
            if (step < 0 || step >= scenario.Steps) return new InputState();
            return scenario.InputAt(slot, step);
        }
    }
}
