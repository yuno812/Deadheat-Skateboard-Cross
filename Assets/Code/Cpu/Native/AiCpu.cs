using System;

namespace Dsc.Ai.Native
{
    // dsc_ai_native.dll のCPUを1体ぶん持つ。使い終わったら Dispose する。
    // DLLの中身は rust/crates/dsc-ai、作り直しは scripts/build-ai.ps1。
    public sealed unsafe class AiCpu : IDisposable
    {
        private static bool layoutChecked;
        private DscAiCpu* cpu;

        public AiCpu(Difficulty difficulty, ulong seed)
        {
            CheckLayout();
            cpu = NativeMethods.dsc_ai_create((byte)difficulty, seed);
            if (cpu == null)
                throw new ArgumentOutOfRangeException(nameof(difficulty), difficulty, "不明な難易度です");
        }

        // 失敗したとき（Rust側のpanic、Dispose後）は入力なしを返す
        public Input Act(Observation obs)
        {
            Input input;
            NativeMethods.dsc_ai_act(cpu, &obs, &input);
            return input;
        }

        public void Dispose()
        {
            Release();
            GC.SuppressFinalize(this);
        }

        ~AiCpu()
        {
            Release();
        }

        private void Release()
        {
            if (cpu == null) return;
            NativeMethods.dsc_ai_destroy(cpu);
            cpu = null;
        }

        // 古いDLLを読み込んでいると構造体の大きさがずれ、壊れた値のまま動いてしまうので最初に止める
        private static void CheckLayout()
        {
            if (layoutChecked) return;
            if (sizeof(Observation) != NativeMethods.dsc_ai_observation_size() ||
                sizeof(Input) != NativeMethods.dsc_ai_input_size())
            {
                throw new InvalidOperationException(
                    "dsc_ai_native.dll とC#の宣言が合っていません。scripts/build-ai.ps1 でDLLを作り直してください");
            }
            layoutChecked = true;
        }
    }
}
