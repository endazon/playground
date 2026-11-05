using Ip.Shared.Protocol;
using Ip.Shared.Simulation;

namespace Ip.Shared.Tests;

/// <summary>
/// プラント (ドライブ) 単独の保護動作。通信もバックエンドも死んだ状態で電圧を出し続けないことを、
/// コントローラの助けなしに保証できているかを確かめる。
/// </summary>
public sealed class PlantProtectionTests
{
    /// <summary>
    /// 実際の駆動ループと同じく小刻みに時間を進める。
    /// 一気に進めると <see cref="VirtualPlant.MaxCatchUpSeconds"/> の時間切り捨てが働いてしまう。
    /// </summary>
    private static void Advance(VirtualPlant plant, double fromMs, double toMs, double stepMs = 10.0)
    {
        for (double t = fromMs + stepMs; t <= toMs + 1e-9; t += stepMs) plant.AdvanceTo(t);
    }

    private static VirtualPlant CreatePlant(out List<MotionEvent> events, out List<EncoderFeedback> feedback)
    {
        var plant = new VirtualPlant(random: new Random(1));
        var motion = new List<MotionEvent>();
        var fb = new List<EncoderFeedback>();
        plant.MotionEventRaised += motion.Add;
        plant.FeedbackReady += fb.Add;
        events = motion;
        feedback = fb;
        return plant;
    }

    [Fact]
    public void Command_IsApplied_AndDrivesTheCart()
    {
        var plant = CreatePlant(out _, out _);
        plant.Reset(upright: false);
        plant.AdvanceTo(0);

        plant.ApplyCommand(new VoltageCommand(1, 0, 2.0, 0.0), 0);
        Advance(plant, 0, 200);

        Assert.True(plant.State.X > 0.01, $"電圧をかけたのに動いていない: x={plant.State.X}");
        Assert.Equal(1, plant.AppliedCommands);
        Assert.True(plant.DriveEnabled);
        Assert.Equal(2.0, plant.AppliedVolts, 9);
    }

    [Fact]
    public void StaleCommandId_IsRejected()
    {
        var plant = CreatePlant(out _, out _);
        plant.AdvanceTo(0);
        plant.ApplyCommand(new VoltageCommand(5, 0, 5.0, 0.0), 0);

        plant.ApplyCommand(new VoltageCommand(4, 99, 20.0, 0.0), 1); // 旧セッション

        Assert.Equal(1, plant.RejectedCommands);
        Assert.Equal(5.0, plant.AppliedVolts, 9);
    }

    [Fact]
    public void SequenceRegression_IsRejected()
    {
        var plant = CreatePlant(out _, out _);
        plant.AdvanceTo(0);
        plant.ApplyCommand(new VoltageCommand(1, 10, 5.0, 0.0), 0);

        plant.ApplyCommand(new VoltageCommand(1, 9, 20.0, 0.0), 1); // 逆行

        Assert.Equal(1, plant.RejectedCommands);
        Assert.Equal(5.0, plant.AppliedVolts, 9);
    }

    [Fact]
    public void EmergencyStop_CutsTheVoltage_AndNotifies()
    {
        var plant = CreatePlant(out var events, out _);
        plant.AdvanceTo(0);
        plant.ApplyCommand(new VoltageCommand(1, 0, 20.0, 0.0), 0);

        plant.EmergencyStop(10);

        Assert.False(plant.DriveEnabled);
        Assert.Equal(0.0, plant.AppliedVolts);
        Assert.Contains(events, e => e.Kind == MotionEventKind.EmergencyStop);
    }

    [Fact]
    public void NewSession_IsRejected_WhileTheDriveIsDisabled()
    {
        var plant = CreatePlant(out var events, out _);
        plant.AdvanceTo(0);
        plant.EmergencyStop(1);
        events.Clear();

        plant.ApplyCommand(new VoltageCommand(2, 0, 20.0, 0.0), 2);

        Assert.Single(events);
        Assert.Equal(MotionEventKind.DriveDisabled, events[0].Kind);
        Assert.Equal(0.0, plant.AppliedVolts);
        Assert.Equal(1, plant.RejectedCommands);
    }

    [Fact]
    public void Reset_ClearsTheSessionGeneration_SoAReconnectedControllerCanDriveAgain()
    {
        // 再接続するとバック側は新しいセッションになり CommandId を 1 から採番し直す。
        // プラントが古い世代を覚えたままだと、その指令を「旧セッション」とみなして全部捨て、
        // ドライブは有効なのに 1 本も適用されない状態がリロードまで続く。
        var plant = CreatePlant(out _, out _);
        plant.AdvanceTo(0);
        for (long id = 1; id <= 3; id++) plant.ApplyCommand(new VoltageCommand(id, 0, 1.0, 0.0), 10 * id);
        Assert.Equal(3, plant.ActiveCommandId);

        plant.Reset(upright: true);

        Assert.Equal(0, plant.ActiveCommandId);
        Assert.Equal(0.0, plant.MeasuredE2EMs, 9);   // 前セッションの実測遅延を持ち越さない

        plant.ApplyCommand(new VoltageCommand(1, 0, 3.0, 0.0), 100);
        Assert.Equal(3.0, plant.AppliedVolts, 9);
    }

    [Fact]
    public void Reset_ReenablesTheDrive_AndBumpsTheEpoch()
    {
        var plant = CreatePlant(out _, out _);
        plant.AdvanceTo(0);
        plant.EmergencyStop(1);
        int epochBefore = plant.Epoch;

        plant.Reset(upright: true);
        plant.ApplyCommand(new VoltageCommand(3, 0, 6.0, 0.0), 2);

        Assert.True(plant.DriveEnabled);
        Assert.False(plant.EmergencyStopped);
        Assert.Equal(epochBefore + 1, plant.Epoch);
        Assert.Equal(6.0, plant.AppliedVolts, 9);
    }

    [Fact]
    public void CommandTimeout_ZeroesTheVoltage_WhenCommandsStop()
    {
        var plant = CreatePlant(out var events, out _);
        plant.AdvanceTo(0);
        plant.ApplyCommand(new VoltageCommand(1, 0, 0.6, 0.0), 0); // レール端に達しない程度の電圧

        Advance(plant, 0, VirtualPlant.CommandTimeoutMs + 50);

        Assert.Contains(events, e => e.Kind == MotionEventKind.CommandTimeout);
        Assert.Equal(0.0, plant.AppliedVolts);
        Assert.True(plant.DriveEnabled, "指令タイムアウトはドライブ自体を殺さない (電圧を 0 にするだけ)");
    }

    [Fact]
    public void MechanicalLimit_DisablesTheDrive()
    {
        var plant = CreatePlant(out var events, out _);
        plant.AdvanceTo(0);

        // 指令を出し続けて端まで走らせる
        long seq = 0;
        for (double t = 0; t < 4000 && plant.DriveEnabled; t += 10)
        {
            plant.ApplyCommand(new VoltageCommand(1, seq++, 24.0, t), t);
            plant.AdvanceTo(t + 10);
        }

        Assert.False(plant.DriveEnabled);
        Assert.Contains(events, e => e.Kind == MotionEventKind.LimitReached);
    }

    [Fact]
    public void Feedback_IsEmittedAtTheConfiguredPeriod()
    {
        var plant = CreatePlant(out _, out var feedback);
        plant.Configure(new NetworkConfig(PeriodMs: 10));
        plant.AdvanceTo(0);

        Advance(plant, 0, 1000);

        Assert.InRange(feedback.Count, 95, 105);
        var intervals = feedback.Zip(feedback.Skip(1), (a, b) => b.TimestampMs - a.TimestampMs).ToList();
        Assert.All(intervals, dt => Assert.InRange(dt, 9.0, 11.0));
    }

    [Fact]
    public void Feedback_CarriesQuantizedEncoderCounts()
    {
        var plant = CreatePlant(out _, out var feedback);
        plant.Reset(upright: true);
        plant.AdvanceTo(0);
        Advance(plant, 0, 100);

        var last = feedback[^1];
        double metersPerCount = plant.State.X / last.CartCounts;

        // 生の物理量ではなくパルス数が送られている (分解能の粒度に乗っている)
        Assert.True(Math.Abs(metersPerCount) < 1e-3);
        Assert.Equal(plant.Epoch, last.Epoch);
    }

    [Fact]
    public void TimeSlip_IsRecorded_WhenTheLoopStallsForTooLong()
    {
        var plant = CreatePlant(out _, out _);
        plant.AdvanceTo(0);

        plant.AdvanceTo(5000); // 5 秒ぶんまとめて要求する = タブが止まっていた状況

        Assert.Equal(1, plant.TimeSlips);
        Assert.True(plant.SimSeconds <= VirtualPlant.MaxCatchUpSeconds + 1e-9);
    }
}
