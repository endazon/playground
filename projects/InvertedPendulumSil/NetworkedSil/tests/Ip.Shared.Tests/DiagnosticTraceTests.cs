using Ip.Shared.Control;
using Ip.Shared.Diagnostics;
using Ip.Shared.Protocol;
using Ip.Shared.Simulation;

namespace Ip.Shared.Tests;

/// <summary>
/// 診断トレースは制御ループの内側から呼ばれる。
/// 「購読者がいない・重要度が低いときに文字列整形のコストが乗らない」ことと、
/// 「コントローラとプラントの節目が実際にトレースへ出る」ことを固定する。
/// </summary>
public sealed class DiagnosticTraceTests
{
    /// <summary>ToString が呼ばれたかどうかを記録する。整形が省かれたことの検出に使う。</summary>
    private sealed class Probe
    {
        public int Formatted { get; private set; }
        public override string ToString()
        {
            Formatted++;
            return "probe";
        }
    }

    [Fact]
    public void WithoutSubscribers_NothingIsFormatted()
    {
        var trace = new DiagnosticTrace("test");
        var probe = new Probe();

        trace.Write(DiagnosticLevel.Error, $"値 {probe}");
        trace.Write(DiagnosticLevel.Error, $"値 {probe}" + $" と {probe}");   // 連結した補間文字列も 1 つのハンドラとして扱われる

        Assert.False(trace.IsEnabled(DiagnosticLevel.Error));
        Assert.Equal(0, probe.Formatted);
    }

    [Fact]
    public void BelowMinimumLevel_NothingIsFormattedOrEmitted()
    {
        var trace = new DiagnosticTrace("test") { MinimumLevel = DiagnosticLevel.Information };
        var events = new List<DiagnosticEvent>();
        trace.Emitted += events.Add;
        var probe = new Probe();

        trace.Write(DiagnosticLevel.Trace, $"帰還 {probe}");
        trace.Write(DiagnosticLevel.Debug, $"集計 {probe}");
        trace.Write(DiagnosticLevel.Information, $"節目 {probe}");

        Assert.Equal(1, probe.Formatted);
        var single = Assert.Single(events);
        Assert.Equal(DiagnosticLevel.Information, single.Level);
        Assert.Equal("test", single.Source);
        Assert.Equal("節目 probe", single.Message);
    }

    [Fact]
    public void FormatIsCultureInvariant()
    {
        var trace = new DiagnosticTrace("test");
        var events = new List<DiagnosticEvent>();
        trace.Emitted += events.Add;

        trace.Write(DiagnosticLevel.Information, $"x={1.5:0.00} n={12345}");

        Assert.Equal("x=1.50 n=12345", Assert.Single(events).Message);
    }

    [Fact]
    public void ControllerCore_TracesModeTransitionsAndFaults()
    {
        var core = new ControllerCore();
        var events = new List<DiagnosticEvent>();
        core.Trace.Emitted += events.Add;
        core.SetNetwork(new NetworkConfig(PeriodMs: 5));

        core.Operate(OperatorAction.Balance, 0);
        core.OnFeedback(new EncoderFeedback(0, core.CommandId, 10, 0, 0, 0, true, 0), 10);
        core.OnMotionEvent(new MotionEvent(MotionEventKind.EmergencyStop, core.CommandId, 20, 0, 0), 20);

        Assert.Equal(ControlMode.Fault, core.Mode);
        Assert.Contains(events, e => e.Level == DiagnosticLevel.Information && e.Message.Contains("Idle → Balance"));
        Assert.Contains(events, e => e.Level == DiagnosticLevel.Information && e.Message.Contains("推定器を初期化"));
        Assert.Contains(events, e => e.Level == DiagnosticLevel.Error && e.Message.Contains("FAULT"));
        Assert.Contains(events, e => e.Level == DiagnosticLevel.Error && e.Message.Contains("Balance → Fault"));
        Assert.All(events, e => Assert.Equal("controller", e.Source));
    }

    [Fact]
    public void ControllerCore_EmitsPerFeedbackTraceOnlyAtTraceLevel()
    {
        var core = new ControllerCore();
        var events = new List<DiagnosticEvent>();
        core.Trace.Emitted += events.Add;
        core.Trace.MinimumLevel = DiagnosticLevel.Debug;
        core.SetNetwork(new NetworkConfig(PeriodMs: 5));
        core.Operate(OperatorAction.Balance, 0);

        for (double t = 10; t < 2500; t += 5)
            core.OnFeedback(new EncoderFeedback((long)t, core.CommandId, t, 0, 0, 5, true, 0), t);

        Assert.DoesNotContain(events, e => e.Level == DiagnosticLevel.Trace);
        // プラント時計で 1 秒ごとの集計は Debug で出る
        Assert.True(events.Count(e => e.Level == DiagnosticLevel.Debug && e.Message.StartsWith("集計")) >= 2);

        events.Clear();
        core.Trace.MinimumLevel = DiagnosticLevel.Trace;
        core.OnFeedback(new EncoderFeedback(10_000, core.CommandId, 2500, 0, 0, 5, true, 0), 2500);

        Assert.Contains(events, e => e.Level == DiagnosticLevel.Trace && e.Message.StartsWith("帰還 Seq 10000"));
        Assert.Contains(events, e => e.Level == DiagnosticLevel.Trace && e.Message.StartsWith("指令 #1/"));
    }

    [Fact]
    public void VirtualPlant_TracesProtectionAndSummaries()
    {
        var plant = new VirtualPlant(random: new Random(1));
        var events = new List<DiagnosticEvent>();
        plant.Trace.Emitted += events.Add;

        plant.Reset(upright: true);
        for (double t = 0; t <= 1500; t += 4) plant.AdvanceTo(t);
        plant.ApplyCommand(new VoltageCommand(1, 0, 3.0, 1400), 1500);
        plant.ApplyCommand(new VoltageCommand(1, 0, 3.0, 1400), 1501);   // Seq の逆行
        plant.EmergencyStop(1502);

        Assert.Contains(events, e => e.Level == DiagnosticLevel.Information && e.Message.StartsWith("リセット"));
        Assert.Contains(events, e => e.Level == DiagnosticLevel.Information && e.Message.StartsWith("積分開始"));
        Assert.Contains(events, e => e.Level == DiagnosticLevel.Debug && e.Message.StartsWith("集計 sim t=1.0s"));
        Assert.Contains(events, e => e.Level == DiagnosticLevel.Information && e.Message.Contains("[最初の指令]"));
        Assert.Contains(events, e => e.Level == DiagnosticLevel.Debug && e.Message.Contains("逆行"));
        Assert.Contains(events, e => e.Level == DiagnosticLevel.Error && e.Message.StartsWith("非常停止"));
        Assert.Contains(events, e => e.Message.Contains("MotionEvent EmergencyStop を送出"));
        Assert.All(events, e => Assert.Equal("plant", e.Source));
    }

    [Fact]
    public void DelayLine_CountsSpikesAndReportsThemAtDebug()
    {
        var trace = new DiagnosticTrace("downlink");
        var events = new List<DiagnosticEvent>();
        trace.Emitted += events.Add;
        var delivered = new List<int>();
        var line = new DelayLine<int>(delivered.Add, new Random(3)) { Trace = trace };
        var network = new NetworkConfig(LatencyMs: 10, SpikePercent: 100);

        line.Push(1, 0, network);
        line.Flush(100);
        line.Flush(200);

        Assert.Equal([1], delivered);
        Assert.Equal(1, line.PushedCount);
        Assert.Equal(1, line.DeliveredCount);
        Assert.Equal(1, line.SpikeCount);
        Assert.Contains(events, e => e.Level == DiagnosticLevel.Debug && e.Message.Contains("再送スパイク"));
        Assert.Contains(events, e => e.Level == DiagnosticLevel.Trace && e.Message.StartsWith("配送"));
        Assert.Contains("スパイク 1", line.DescribeStatistics());
    }
}
