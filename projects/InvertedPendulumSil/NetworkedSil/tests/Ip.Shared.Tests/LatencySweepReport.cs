using Ip.Shared.Protocol;
using Xunit.Abstractions;

namespace Ip.Shared.Tests;

/// <summary>
/// 遅延を振って倒立が崩れる境目を表にする。設計メモに載せる数字はここから取る。
/// 併せて「補償なしより予測器ありの方が耐えられる」ことを検証する。
/// </summary>
public sealed class LatencySweepReport(ITestOutputHelper output)
{
    [Fact]
    public void SweepLatency()
    {
        double lastSurvivedWithout = -1, lastSurvivedWith = -1;

        foreach (bool predict in new[] { false, true })
            foreach (int oneWay in new[] { 0, 10, 20, 25, 30, 35, 40, 45, 50, 60, 70, 80 })
            {
                var harness = new SilHarness(
                    new NetworkConfig(PeriodMs: 5, LatencyMs: oneWay),
                    new ControlOptions(UsePredictor: predict));
                harness.StartBalance();

                double next = harness.NowMs + 2000;
                int dir = 1;
                bool survived = true;
                double until = harness.NowMs + 12000;
                while (harness.NowMs < until)
                {
                    harness.Step();
                    if (harness.NowMs >= next)
                    {
                        harness.Plant.Push(dir);
                        dir = -dir;
                        next += 2000;
                    }
                    if (harness.Controller.Mode == ControlMode.Fault) { survived = false; break; }
                }

                if (survived)
                {
                    if (predict) lastSurvivedWith = oneWay * 2; else lastSurvivedWithout = oneWay * 2;
                }

                output.WriteLine($"predict={predict,-5} 片道={oneWay,3}ms 往復={oneWay * 2,3}ms → " +
                    $"{(survived ? "維持" : "転倒")} mode={harness.Controller.Mode} " +
                    $"e2e={harness.Controller.Snapshot().E2EDelayMs:0.0}ms");
            }

        output.WriteLine($"維持できた往復遅延の上限: 補償なし {lastSurvivedWithout}ms / 予測器あり {lastSurvivedWith}ms");
        Assert.True(lastSurvivedWithout > 0, "遅延ゼロでも倒立できていない");
        Assert.True(lastSurvivedWith > lastSurvivedWithout, "予測器を入れても耐えられる遅延が伸びていない");
    }
}
