using System;
using Dsc.Ai.Native;
using NUnit.Framework;

namespace Dsc.Ai.Tests
{
    // dsc_ai_native.dll をUnity（Mono）から呼べるかの確認。判断ロジックの中身はRust側でテストする。
    public class AiCpuTests
    {
        [TestCase(Difficulty.Weak)]
        [TestCase(Difficulty.Normal)]
        [TestCase(Difficulty.Strong)]
        public void CreateAndAct(Difficulty difficulty)
        {
            // 構造体の大きさがDLLと合わなければ、ここで InvalidOperationException になる
            using (var cpu = new AiCpu(difficulty, 42))
            {
                var obs = new Observation
                {
                    step = 7,
                    me = new BodyObs { facing = 1, pose = Pose.Charge },
                    opp = new BodyObs { facing = -1 },
                };
                Assert.AreEqual(default(Input), cpu.Act(obs));
            }
        }

        [Test]
        public void UnknownDifficultyThrows()
        {
            Assert.Throws<ArgumentOutOfRangeException>(() => new AiCpu((Difficulty)9, 0));
        }

        [Test]
        public void ActAfterDisposeReturnsNoInput()
        {
            var cpu = new AiCpu(Difficulty.Normal, 1);
            cpu.Dispose();
            Assert.AreEqual(default(Input), cpu.Act(default));
        }
    }
}
