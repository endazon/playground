using Ip.Shared.Control;
using Ip.Shared.Model;
using Ip.Shared.Protocol;

namespace Ip.Shared.Tests;

/// <summary>
/// 純 SIL ループ (プラント ↔ 通信路 ↔ コントローラ) の結合試験。
/// 設計メモの主張 —— 往復 70ms までは維持、90ms で転倒、予測器を使えば 140ms でも維持 —— を検証する。
/// </summary>
public sealed class DelayedLoopIntegrationTests
{
    private const int PeriodMs = 5;

    /// <param name="oneWayLatencyMs">片道遅延</param>
    /// <param name="predict">予測器を使うか</param>
    /// <param name="durationMs">外乱を加えながら回す時間</param>
    private static (bool Survived, SilHarness Harness) RunWithDisturbances(
        double oneWayLatencyMs, bool predict = false, double durationMs = 12_000)
    {
        var harness = new SilHarness(
            new NetworkConfig(PeriodMs: PeriodMs, LatencyMs: oneWayLatencyMs),
            new ControlOptions(UsePredictor: predict));
        harness.StartBalance();

        double nextPush = harness.NowMs + 2000;
        double until = harness.NowMs + durationMs;
        int direction = 1;

        while (harness.NowMs < until)
        {
            harness.Step();
            if (harness.NowMs >= nextPush)
            {
                harness.Plant.Push(direction);
                direction = -direction;
                nextPush += 2000;
            }
            if (harness.Controller.Mode == ControlMode.Fault) return (false, harness);
        }
        return (harness.IsBalancing, harness);
    }

    [Theory]
    [InlineData(0)]
    [InlineData(10)]
    [InlineData(20)]
    [InlineData(35)] // 往復 70ms: 設計メモが「維持できた」とした条件
    public void BalanceSurvives_WhenRoundTripIsInsideTheDelayMargin(double oneWayLatencyMs)
    {
        var (survived, harness) = RunWithDisturbances(oneWayLatencyMs);

        Assert.True(survived, $"片道 {oneWayLatencyMs}ms で転倒した: {harness.Controller.Reason}");
        Assert.Equal(ControlMode.Balance, harness.Controller.Mode);
    }

    [Fact]
    public void BalanceFalls_WhenRoundTripExceedsTheDelayMargin()
    {
        // 理論遅延余裕は 85ms。往復 90ms は余裕を超えるので、外乱を受ければ倒れる。
        var (survived, harness) = RunWithDisturbances(45);

        Assert.False(survived);
        Assert.Equal(ControlMode.Fault, harness.Controller.Mode);
        Assert.Contains("転倒検知", harness.Controller.Reason);
    }

    [Fact]
    public void MeasuredBoundary_AgreesWithTheTheoreticalDelayMargin()
    {
        int margin = ControllerCore.CreateDesign(new PendulumParameters(), PeriodMs, 25.0).Info.DelayMarginMs;

        int lastSurvived = 0;
        for (int oneWay = 0; oneWay <= 60; oneWay += 5)
        {
            if (!RunWithDisturbances(oneWay, durationMs: 8000).Survived) break;
            lastSurvived = oneWay * 2;
        }

        // 実測の境界が理論値から大きく外れていたら、実装かモデルのどちらかが壊れている
        Assert.InRange(lastSurvived, margin - 25, margin + 25);
    }

    [Fact]
    public void Predictor_ExtendsTheUsableDelay()
    {
        Assert.False(RunWithDisturbances(70).Survived);                        // 往復 140ms: 補償なしでは倒れる
        Assert.True(RunWithDisturbances(70, predict: true).Survived);          // 予測器ありなら維持できる
    }

    [Fact]
    public void SwingUp_ReachesBalanceFromTheHangingPosition()
    {
        var harness = new SilHarness(new NetworkConfig(PeriodMs: PeriodMs));
        harness.StartSwingUp();

        harness.Run(20_000);

        Assert.Equal(ControlMode.Balance, harness.Controller.Mode);
        Assert.True(Math.Abs(CartPoleDynamics.WrapAngle(harness.Plant.State.Theta)) < 0.2,
            $"倒立していない: θ={harness.Plant.State.Theta:0.000}");
    }

    [Fact]
    public void ControllerEstimate_TracksThePlantWithinTheDelay()
    {
        var (survived, harness) = RunWithDisturbances(10, durationMs: 6000);
        Assert.True(survived);

        var status = harness.Controller.Snapshot();

        // バックが見ている状態は遅れているが、倒立中なら数 mm / 数度のオーダーに収まる
        Assert.True(Math.Abs(status.EstimatedX - harness.Plant.State.X) < 0.05);
        Assert.True(Math.Abs(status.EstimatedTheta - CartPoleDynamics.WrapAngle(harness.Plant.State.Theta)) < 0.1);
        Assert.InRange(status.FeedbackIntervalMs, PeriodMs - 1.0, PeriodMs + 1.0);
        Assert.InRange(status.E2EDelayMs, 15.0, 30.0); // 片道 10ms → 往復 20ms + 量子化
    }
}
