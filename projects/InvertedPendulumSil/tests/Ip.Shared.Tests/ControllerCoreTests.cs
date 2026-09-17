using Ip.Shared.Control;
using Ip.Shared.Protocol;

namespace Ip.Shared.Tests;

/// <summary>
/// コントローラ単体の安全側の挙動。結合テストでは通信が正常なことが前提になるため、
/// 「壊れた入力」「時計が合っていない」といった状況はここで固定する。
/// </summary>
public sealed class ControllerCoreTests
{
    private const int PeriodMs = 5;

    private sealed class Harness
    {
        public ControllerCore Core { get; } = new();
        public List<VoltageCommand> Commands { get; } = [];
        public List<AbortCommand> Aborts { get; } = [];
        public List<LogEntry> Logs { get; } = [];
        public long Seq { get; private set; }

        public Harness()
        {
            Core.SetNetwork(new NetworkConfig(PeriodMs));
            Core.VoltageProduced += Commands.Add;
            Core.AbortProduced += Aborts.Add;
            Core.Logged += Logs.Add;
        }

        /// <summary>倒立付近の帰還を 1 本送る。</summary>
        public void Feed(double plantTimeMs, double nowMs, bool driveEnabled = true, double theta = 0.01,
            double e2eMs = 10.0)
        {
            var p = Core.Parameters;
            long pendulumCounts = (long)Math.Round(theta / (2.0 * Math.PI) * p.PendulumEncoderCountsPerRev);
            Core.OnFeedback(
                new EncoderFeedback(Seq++, Core.CommandId, plantTimeMs, 0, pendulumCounts, e2eMs, driveEnabled, 0),
                nowMs);
        }
    }

    [Fact]
    public void Stop_DoesNotClearAFault()
    {
        // 「停止」で異常が消えると、ログにも残らないまま FAULT が無かったことになる
        var h = new Harness();
        h.Core.Operate(OperatorAction.Balance, 0);
        h.Feed(plantTimeMs: 10, nowMs: 10);
        h.Core.OnMotionEvent(new MotionEvent(MotionEventKind.EmergencyStop, h.Core.CommandId, 20, 0, 0), 20);
        Assert.Equal(ControlMode.Fault, h.Core.Mode);

        h.Core.Operate(OperatorAction.Stop, 30);

        Assert.Equal(ControlMode.Fault, h.Core.Mode);

        h.Core.Operate(OperatorAction.ClearFault, 40);
        Assert.Equal(ControlMode.Idle, h.Core.Mode);
    }

    [Fact]
    public void DriveDisabledInFeedback_FaultsTheController()
    {
        // MotionEvent が遅れても、定周期帰還のフラグでドライブ遮断に気づけること
        var h = new Harness();
        h.Core.Operate(OperatorAction.Balance, 0);
        h.Feed(plantTimeMs: 10, nowMs: 10);
        Assert.Equal(ControlMode.Balance, h.Core.Mode);

        h.Feed(plantTimeMs: 15, nowMs: 15, driveEnabled: false);

        Assert.Equal(ControlMode.Fault, h.Core.Mode);
        Assert.Contains("ドライブ無効", h.Core.Reason);
        Assert.NotEmpty(h.Aborts);
    }

    [Fact]
    public void Watchdog_FiresEvenWhenWatchdogMsArrivedAsNaN()
    {
        // NaN が Math.Clamp を素通りすると Math.Max(NaN, x) も NaN になり、
        // 「nowMs - last > NaN」が常に false になってウォッチドッグが永久に沈黙する
        var h = new Harness();
        h.Core.SetOptions(new ControlOptions(WatchdogMs: double.NaN));
        h.Core.Operate(OperatorAction.Balance, 0);
        h.Feed(plantTimeMs: 10, nowMs: 10);

        h.Core.Tick(10_000);

        Assert.Equal(ControlMode.Fault, h.Core.Mode);
        Assert.Contains("ウォッチドッグ", h.Core.Reason);
    }

    [Fact]
    public void FeedbackMeasuredBeforeStart_IsDropped_AndTheWatchdogStillFires()
    {
        // 古い計測を捨てること自体は正しいが、捨て続けている間にウォッチドッグまで黙ると
        // 「BALANCE のまま電圧を 1 本も出さない」サイレント故障になる
        var h = new Harness();
        h.Feed(plantTimeMs: 1000, nowMs: 1000);        // IDLE 中に現在のプラント時刻を知らせる
        h.Core.Operate(OperatorAction.Balance, 1001);
        h.Commands.Clear();

        for (double t = 1002; t < 1400; t += 5) h.Feed(plantTimeMs: 500, nowMs: t);  // 開始前の計測ばかり届く

        Assert.Empty(h.Commands);
        h.Core.Tick(1500);
        Assert.Equal(ControlMode.Fault, h.Core.Mode);
        Assert.Contains("ウォッチドッグ", h.Core.Reason);
    }

    [Fact]
    public void ControlDoesNotDependOnTheClockOffset()
    {
        // 時計オフセットの推定は「上り遅延」の表示にしか使わない
        var h = new Harness();
        h.Core.ClockOffsetMs = -1_000_000.0;   // でたらめな推定値
        h.Core.Operate(OperatorAction.Balance, 0);

        for (double t = 10; t < 60; t += 5) h.Feed(plantTimeMs: t, nowMs: t);

        Assert.Equal(ControlMode.Balance, h.Core.Mode);
        Assert.NotEmpty(h.Commands);
    }

    [Fact]
    public void Predictor_ClampsTheExtrapolationHorizon()
    {
        // E2E 遅延がスパイクした値をそのまま信じると、不安定モデルで何百 ms も開ループ外挿する
        double atLimit = RunWithMeasuredDelay(ControllerCore.MaxPredictionHorizonMs);
        double absurd = RunWithMeasuredDelay(5_000.0);

        Assert.Equal(atLimit, absurd, 9);

        static double RunWithMeasuredDelay(double e2eMs)
        {
            var h = new Harness();
            h.Core.SetOptions(new ControlOptions(UsePredictor: true));
            h.Core.Operate(OperatorAction.Balance, 0);
            for (double t = 10; t < 400; t += PeriodMs) h.Feed(plantTimeMs: t, nowMs: t, e2eMs: e2eMs);
            return h.Commands[^1].Volts;
        }
    }

    [Fact]
    public void Snapshot_SmoothsTheUplinkDelayEvenWhenItStartsAtZero()
    {
        // 初期化判定を「値が 0 か」で行うと、遅延 0 の構成で平滑が一切効かない
        var h = new Harness();
        h.Core.Operate(OperatorAction.Balance, 0);
        for (double t = 10; t < 60; t += 5) h.Feed(plantTimeMs: t, nowMs: t);   // 上り遅延 0
        h.Feed(plantTimeMs: 60, nowMs: 160);                                    // 突然 100ms

        // 平滑が効いていれば、1 サンプルで 100ms には飛ばない
        Assert.InRange(h.Core.Snapshot().UplinkDelayMs, 1.0, 40.0);
    }
}
