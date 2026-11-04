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

        private AiCpu(DscAiCpu* cpu)
        {
            this.cpu = cpu;
        }

        // 対戦用のCPU。character はキャラ名（Prefab のファイル名の小文字）、調整済みの値はDLLに埋め込み。
        // polygonXY はステージの当たり判定の凸多角形の頂点（ワールド座標、x0, y0, x1, y1, …）、
        // polygonCounts は多角形ごとの頂点数
        public static AiCpu CreateBrain(string character, Difficulty difficulty, ulong seed, BrainSetup setup, float[] polygonXY, uint[] polygonCounts)
        {
            CheckLayout();
            byte[] name = System.Text.Encoding.UTF8.GetBytes(character ?? "");
            DscAiCpu* cpu;
            fixed (byte* namePtr = name)
            fixed (float* xy = polygonXY)
            fixed (uint* counts = polygonCounts)
            {
                cpu = NativeMethods.dsc_ai_create_brain(
                    namePtr, (nuint)name.Length, (byte)difficulty, seed, &setup, xy, counts, (uint)polygonCounts.Length);
            }
            if (cpu == null)
                throw new InvalidOperationException($"CPU を作れませんでした（{character}, {difficulty}）");
            return new AiCpu(cpu);
        }

        // 失敗したとき（Rust側のpanic、Dispose後）は入力なしを返す
        public Input Act(Observation obs)
        {
            Input input;
            NativeMethods.dsc_ai_act(cpu, &obs, &input);
            return input;
        }

        // 今のハンデ（反応の遅れのステップ数、狙いのブレ、もたつきの確率）。対戦用の CPU でなければ false
        public bool TryGetHandicap(out int reactionDelaySteps, out float aimNoise, out float slip)
        {
            uint delay;
            float aim, s;
            bool ok = NativeMethods.dsc_ai_get_handicap(cpu, &delay, &aim, &s);
            reactionDelaySteps = (int)delay;
            aimNoise = aim;
            slip = s;
            return ok;
        }

        // 試合の途中でハンデを変える（遊びながら難易度の値を決めるため）
        public bool SetHandicap(int reactionDelaySteps, float aimNoise, float slip)
        {
            return NativeMethods.dsc_ai_set_handicap(cpu, (uint)System.Math.Max(0, reactionDelaySteps), aimNoise, slip);
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
