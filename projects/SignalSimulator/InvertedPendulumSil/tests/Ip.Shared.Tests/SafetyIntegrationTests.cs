using Ip.Shared.Protocol;

namespace Ip.Shared.Tests;

/// <summary>監視の多重化 (バック側の検知とプラント側の保護) が、結合状態でも意図通りに働くことの確認。</summary>
public sealed class SafetyIntegrationTests
{
    [Fact]
    public void EmergencyStop_FaultsTheController()
    {
        var harness = new SilHarness(new NetworkConfig(PeriodMs: 5, LatencyMs: 5));
        harness.StartBalance();
        harness.Run(1000);
        Assert.Equal(ControlMode.Balance, harness.Controller.Mode);

        harness.Plant.EmergencyStop(harness.NowMs);
        harness.Run(200);

        Assert.Equal(ControlMode.Fault, harness.Controller.Mode);
        Assert.Contains("非常停止", harness.Controller.Reason);
        Assert.False(harness.Plant.DriveEnabled);
    }

    [Fact]
    public void ClearFaultAlone_DoesNotRecover_BecauseThePlantStillRefuses()
    {
        // 「異常リセット」はバック側の確認にすぎない。現場側のドライブは無効のままなので運転は再開しない。
        var harness = new SilHarness(new NetworkConfig(PeriodMs: 5, LatencyMs: 5));
        harness.StartBalance();
        harness.Run(500);
        harness.Plant.EmergencyStop(harness.NowMs);
        harness.Run(200);
        Assert.Equal(ControlMode.Fault, harness.Controller.Mode);

        harness.Controller.Operate(OperatorAction.ClearFault, harness.NowMs);
        Assert.Equal(ControlMode.Idle, harness.Controller.Mode);

        harness.Controller.Operate(OperatorAction.Balance, harness.NowMs);
        harness.Run(300);

        Assert.Equal(ControlMode.Fault, harness.Controller.Mode);
        Assert.Contains("ドライブ無効", harness.Controller.Reason);
    }

    [Fact]
    public void PlantInitialization_AllowsTheOperatorToRestart()
    {
        var harness = new SilHarness(new NetworkConfig(PeriodMs: 5, LatencyMs: 5));
        harness.StartBalance();
        harness.Run(500);
        harness.Plant.EmergencyStop(harness.NowMs);
        harness.Run(200);
        harness.Controller.Operate(OperatorAction.ClearFault, harness.NowMs);

        harness.StartBalance(); // プラント初期化 → 開始
        harness.Run(1500);

        Assert.Equal(ControlMode.Balance, harness.Controller.Mode);
        Assert.True(harness.Plant.DriveEnabled);
    }

    [Fact]
    public void Watchdog_FaultsWhenFeedbackStops()
    {
        var harness = new SilHarness(new NetworkConfig(PeriodMs: 5));
        harness.StartBalance();
        harness.Run(500);

        harness.UplinkEnabled = false;
        harness.Run(600);

        Assert.Equal(ControlMode.Fault, harness.Controller.Mode);
        Assert.Contains("ウォッチドッグ", harness.Controller.Reason);
    }

    [Fact]
    public void DownlinkLoss_TripsThePlantCommandTimeout()
    {
        var harness = new SilHarness(new NetworkConfig(PeriodMs: 5));
        harness.StartBalance();
        harness.Run(500);

        // 下りだけ切る: バックは帰還を受け続けるのでウォッチドッグは働かない。
        // それでもプラント側の指令タイムアウトが電圧を 0 にする。
        harness.DownlinkEnabled = false;
        harness.Run(700);

        Assert.Equal(0.0, harness.Plant.AppliedVolts);
        Assert.Equal(ControlMode.Fault, harness.Controller.Mode);
    }

    [Fact]
    public void StaleMotionEvent_FromAPreviousSession_IsDiscarded()
    {
        var harness = new SilHarness(new NetworkConfig(PeriodMs: 5));
        harness.StartBalance();
        harness.Run(300);
        long currentCommandId = harness.Controller.CommandId;

        harness.Controller.OnMotionEvent(
            new MotionEvent(MotionEventKind.LimitReached, currentCommandId - 1, harness.NowMs, 0, 0),
            harness.NowMs);

        Assert.Equal(ControlMode.Balance, harness.Controller.Mode);
        Assert.Equal(1, harness.Controller.Snapshot().StaleEventCount);
    }

    [Fact]
    public void Start_IsRefused_WhileFaulted()
    {
        var harness = new SilHarness(new NetworkConfig(PeriodMs: 5));
        harness.StartBalance();
        harness.Run(300);
        harness.Plant.EmergencyStop(harness.NowMs);
        harness.Run(200);
        long commandIdAtFault = harness.Controller.CommandId;

        harness.Controller.Operate(OperatorAction.SwingUp, harness.NowMs);

        Assert.Equal(ControlMode.Fault, harness.Controller.Mode);
        Assert.Equal(commandIdAtFault, harness.Controller.CommandId);
        Assert.Contains(harness.Log, l => l.Message.Contains("異常リセットが必要"));
    }

    [Fact]
    public void Stop_ReturnsToIdle_AndReleasesThePlant()
    {
        var harness = new SilHarness(new NetworkConfig(PeriodMs: 5));
        harness.StartBalance();
        harness.Run(500);

        harness.Controller.Operate(OperatorAction.Stop, harness.NowMs);
        harness.Run(100);

        Assert.Equal(ControlMode.Idle, harness.Controller.Mode);
        Assert.Equal(0.0, harness.Plant.AppliedVolts);
    }

    [Fact]
    public void PlantReset_DuringOperation_ReinitializesTheEstimator()
    {
        // epoch が変わったのに古い計測との差分で速度を作ると、巨大な速度推定値が出て暴れる
        var harness = new SilHarness(new NetworkConfig(PeriodMs: 5));
        harness.StartBalance();
        harness.Run(1000);

        harness.Plant.Reset(upright: true);
        harness.Run(300);

        var status = harness.Controller.Snapshot();
        Assert.True(Math.Abs(status.EstimatedX) < 0.1);
        Assert.True(Math.Abs(status.EstimatedTheta) < 0.3);
    }
}
