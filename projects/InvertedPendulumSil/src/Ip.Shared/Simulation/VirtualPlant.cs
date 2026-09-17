using Ip.Shared.Diagnostics;
using Ip.Shared.Model;
using Ip.Shared.Protocol;

namespace Ip.Shared.Simulation;

/// <summary>
/// 仮想ハードウェア。1ms 固定ステップの RK4 で台車・振子・モータを積分し、
/// 仮想エンコーダのパルスを定周期で上りに流す。
///
/// 実機のドライブに相当する保護をここに閉じ込めているのが要点である:
/// 非常停止・メカリミット・指令タイムアウト・無効中の指令拒否。
/// 通信もバックエンドも死んだ状態で電圧を出し続けないことを、プラント単独で保証する。
///
/// <see cref="AdvanceTo"/> に渡す時刻を実時間にすれば実時間シミュレーション、
/// 仮想時間にすれば完全再現性のあるロックステップ試験になる。
///
/// 動作の可視化のため診断トレース <see cref="Trace"/> を持つ。
/// 帰還 1 本・指令 1 本ごとの内容は Trace、1 秒ごとの集計と指令の破棄は Debug、
/// 保護動作・リセット・設定変更は Information 以上で出す。
/// </summary>
public sealed class VirtualPlant
{
    /// <summary>固定積分ステップ [s]</summary>
    public const double StepSeconds = 0.001;
    /// <summary>指令が途絶えたときに電圧を切るまでの時間 [ms]</summary>
    public const double CommandTimeoutMs = 500.0;
    /// <summary>1 回の <see cref="AdvanceTo"/> で許す最大の追いつき時間 [s]。これを超えたぶんは捨てる。</summary>
    public const double MaxCatchUpSeconds = 0.2;
    /// <summary>診断トレースの集計を出す間隔 (シミュレーション時刻) [s]</summary>
    public const double SummaryIntervalSeconds = 1.0;
    /// <summary>
    /// メカリミット検出のしきい値に使う、ストッパの静的たわみに対する倍率。
    /// しきい値を静的たわみと同値にすると、最大電圧で準静的に押し付けたときに
    /// ちょうど届かず、リミットが永久に出ないことがある。
    /// </summary>
    private const double LimitDetectionFactor = 0.5;
    /// <summary>外乱ボタンで与える軸トルク [Nm] とその継続時間 [s]</summary>
    private const double DisturbanceTorque = 0.3;
    private const double DisturbanceSeconds = 0.05;
    private const double RadToDeg = 180.0 / Math.PI;

    private readonly Random _random;
    private PendulumParameters _parameters;
    private DerivedParameters _derived;
    private NetworkConfig _network = new();

    private PlantState _state = PlantState.Hanging;
    private double _simSeconds;
    private double _originMs;
    private bool _originSet;

    private double _volts;
    private long _activeCommandId;
    private long _lastSeq = -1;
    private bool _sessionAccepted = true;
    private bool _commandRunning;
    private double _lastCommandAtMs;
    private bool _timeoutFired;

    private double _nextFeedbackMs;
    private long _feedbackSeq;
    private Disturbance _disturbance = Disturbance.None;
    private double _disturbanceEndSeconds = -1.0;

    // ---- 診断トレース用の集計 (物理にもプロトコルにも使わない) ----
    private long _rejectedSinceLastApplied;
    private double _nextSummaryAtSeconds = SummaryIntervalSeconds;
    private long _stepsAtSummary;
    private long _feedbackAtSummary;
    private long _appliedAtSummary;
    private long _rejectedAtSummary;
    private double _maxAbsVoltsSinceSummary;
    private double _maxAbsXDotSinceSummary;
    private int _advanceCallsSinceSummary;
    private int _maxStepsPerAdvance;
    private double _maxLagMsSinceSummary;

    public VirtualPlant(PendulumParameters? parameters = null, Random? random = null)
    {
        _parameters = parameters ?? new PendulumParameters();
        _derived = _parameters.Derive();
        _random = random ?? Random.Shared;
    }

    /// <summary>定周期の帰還。呼び出し側が通信路へ流す。引数は計測時刻 (プラント壁時計) 付き。</summary>
    public event Action<EncoderFeedback>? FeedbackReady;

    /// <summary>保護動作の即時通知。</summary>
    public event Action<MotionEvent>? MotionEventRaised;

    /// <summary>診断トレース。ホスト側でログ基盤へつなぐ。購読者がいなければ何も整形しない。</summary>
    public DiagnosticTrace Trace { get; } = new("plant");

    public PlantState State => _state;

    /// <summary>
    /// メカリミットとしてドライブを遮断する台車位置 [m]。
    /// 最大電圧で押し付けたときのストッパの静的たわみの半分だけレール端より外に置く。
    /// </summary>
    public double LimitTriggerPosition =>
        _parameters.RailStroke
        + LimitDetectionFactor * _derived.ForceGain * _parameters.MaxVoltage / _parameters.StopStiffness;
    public double SimSeconds => _simSeconds;
    public bool DriveEnabled { get; private set; } = true;
    public bool EmergencyStopped { get; private set; }
    /// <summary>実際に印加されている電圧 [V]。ドライブ無効なら 0。</summary>
    public double AppliedVolts => DriveEnabled ? _volts : 0.0;
    /// <summary>リセット世代。コントローラはこれが変わったら推定器を初期化する。</summary>
    public int Epoch { get; private set; }
    /// <summary>破棄した指令の数 (古い CommandId / 逆行した Seq / 無効中)</summary>
    public long RejectedCommands { get; private set; }
    public long AppliedCommands { get; private set; }
    /// <summary>実測した「計測 → 指令適用」遅延 [ms]</summary>
    public double MeasuredE2EMs { get; private set; }
    /// <summary>実時間に追いつけず時間を捨てた回数。タブ非アクティブなどで起きる。</summary>
    public int TimeSlips { get; private set; }
    public long ActiveCommandId => _activeCommandId;
    /// <summary>これまでに積分した固定ステップの数。</summary>
    public long StepCount { get; private set; }
    /// <summary>これまでに送出した帰還の数。</summary>
    public long FeedbackCount => _feedbackSeq;

    public void Configure(NetworkConfig config)
    {
        config = config.Sanitized();
        bool periodChanged = config.PeriodMs != _network.PeriodMs;
        var previous = _network;
        _network = config;
        if (periodChanged) _nextFeedbackMs = _simSeconds * 1000.0; // 次のステップで 1 回送ってから新周期に乗る

        Trace.Write(config == previous ? DiagnosticLevel.Debug : DiagnosticLevel.Information,
            $"通信条件: 帰還周期 {previous.PeriodMs}→{config.PeriodMs} ms, エンコーダノイズ σ{config.EncoderNoiseCounts:0.0} count " +
            $"(遅延 {config.LatencyMs:0} ms / ジッタ {config.JitterMs:0} ms / スパイク {config.SpikePercent:0.0} % は上り遅延線が使う)" +
            $"{(config == previous ? " (変更なし)" : periodChanged ? " → 次のステップで帰還を 1 本送ってから新周期に乗る" : "")}");
    }

    public void Configure(PlantConfig config)
    {
        config = config.Sanitized();
        if (Math.Abs(config.PendulumLength - _parameters.PendulumLength) < 1e-12)
        {
            Trace.Write(DiagnosticLevel.Debug, $"プラント設定: 振子長 {config.PendulumLength:0.00} m (変更なし)");
            return;
        }
        double previous = _parameters.PendulumLength;
        _parameters = _parameters with { PendulumLength = config.PendulumLength };
        _derived = _parameters.Derive();
        Trace.Write(DiagnosticLevel.Information,
            $"プラント設定: 振子長 {previous:0.00}→{config.PendulumLength:0.00} m (軸まわり慣性 {_derived.PivotInertia:0.0000} kg·m², リミット位置 ±{LimitTriggerPosition:0.000} m)");
    }

    /// <summary>
    /// 下りの電圧指令を適用する。実機のドライブが行う受け入れ判定をここで再現している。
    /// </summary>
    public void ApplyCommand(VoltageCommand command, double nowMs)
    {
        if (command.CommandId < _activeCommandId)
        {
            Reject(command, $"旧セッション #{command.CommandId} < 現行 #{_activeCommandId}");
            return;
        }

        if (command.CommandId > _activeCommandId)
        {
            // 新しい運転セッション。ドライブが無効のままならセッションごと拒否して通知する
            Trace.Write(DriveEnabled ? DiagnosticLevel.Information : DiagnosticLevel.Warning,
                $"新しい運転セッション #{command.CommandId} (前 #{_activeCommandId}) の最初の指令を受信 → " +
                $"{(DriveEnabled ? "受理" : "ドライブ無効のためセッションごと拒否して DriveDisabled を通知")}");
            _activeCommandId = command.CommandId;
            _lastSeq = -1;
            _timeoutFired = false;
            _sessionAccepted = DriveEnabled;
            if (!DriveEnabled) RaiseMotionEvent(MotionEventKind.DriveDisabled, nowMs);
        }

        if (!_sessionAccepted)
        {
            Reject(command, "このセッションはドライブ無効中に始まったので受理しない");
            return;
        }
        if (!DriveEnabled)
        {
            Reject(command, "ドライブ無効中");
            return;
        }
        if (command.Seq <= _lastSeq)
        {
            Reject(command, $"Seq {command.Seq} が直近 {_lastSeq} 以下 (逆行)");
            return;
        }

        if (_rejectedSinceLastApplied > 0)
        {
            Trace.Write(DiagnosticLevel.Debug,
                $"指令の拒否が {_rejectedSinceLastApplied} 件続いた後、#{command.CommandId}/{command.Seq} を再び受理");
            _rejectedSinceLastApplied = 0;
        }

        _lastSeq = command.Seq;
        _volts = CartPoleDynamics.Clamp(command.Volts, -_parameters.MaxVoltage, _parameters.MaxVoltage);
        _lastCommandAtMs = nowMs;
        MeasuredE2EMs = nowMs - command.BasedOnMs;    // 同じプラント時計の差なので時計ズレの影響を受けない
        _commandRunning = true;
        AppliedCommands++;
        _maxAbsVoltsSinceSummary = Math.Max(_maxAbsVoltsSinceSummary, Math.Abs(_volts));

        if (AppliedCommands == 1 || Trace.IsEnabled(DiagnosticLevel.Trace))
        {
            Trace.Write(AppliedCommands == 1 ? DiagnosticLevel.Information : DiagnosticLevel.Trace,
                $"指令 #{command.CommandId}/{command.Seq} を適用: {_volts:+0.000;-0.000;+0.000} V, E2E {MeasuredE2EMs:0.0} ms (計測 {command.BasedOnMs:0.0} → 適用 {nowMs:0.0})" +
                $"{(AppliedCommands == 1 ? " [最初の指令]" : "")}");
        }
    }

    public void ApplyAbort(AbortCommand command)
    {
        if (command.CommandId < _activeCommandId)
        {
            Trace.Write(DiagnosticLevel.Debug, $"旧セッション #{command.CommandId} の Abort を無視 (現行 #{_activeCommandId})");
            return;
        }
        Trace.Write(DiagnosticLevel.Information,
            $"Abort #{command.CommandId} を適用: 電圧 {_volts:+0.00;-0.00;+0.00}→0 V, 指令監視を停止 (適用済み {AppliedCommands} 本)");
        _activeCommandId = command.CommandId;
        _volts = 0.0;
        _commandRunning = false;
    }

    /// <summary>現場側の復帰操作。ドライブを再び有効にして状態を初期姿勢へ戻す。</summary>
    public void Reset(bool upright)
    {
        var before = _state;
        _state = upright ? PlantState.NearUpright() : PlantState.Hanging;
        _volts = 0.0;
        _commandRunning = false;
        DriveEnabled = true;
        EmergencyStopped = false;
        _sessionAccepted = true;
        _timeoutFired = false;
        _disturbance = Disturbance.None;
        _disturbanceEndSeconds = -1.0;

        // 運転セッションの世代も初期化する。
        // これを残すと、コントローラ側が再接続で CommandId を採番し直したときに
        // 「古い指令」とみなして新しいセッションの指令を全部捨て続ける。
        long previousCommandId = _activeCommandId;
        _activeCommandId = 0;
        _lastSeq = -1;
        MeasuredE2EMs = 0.0;
        _rejectedSinceLastApplied = 0;

        Epoch++;
        Trace.Write(DiagnosticLevel.Information,
            $"リセット ({(upright ? "倒立姿勢" : "吊り下げ姿勢")}): Epoch {Epoch - 1}→{Epoch}, " +
            $"x={before.X:+0.000;-0.000;+0.000}→{_state.X:+0.000;-0.000;+0.000} m, θ={CartPoleDynamics.WrapAngle(before.Theta) * RadToDeg:+0.0;-0.0;+0.0}→{CartPoleDynamics.WrapAngle(_state.Theta) * RadToDeg:+0.0;-0.0;+0.0}°, " +
            $"ドライブ有効, 非常停止解除, CommandId #{previousCommandId}→#0 (sim t={_simSeconds:0.000} s)");
    }

    public void EmergencyStop(double nowMs)
    {
        if (!DriveEnabled && EmergencyStopped)
        {
            Trace.Write(DiagnosticLevel.Debug, "非常停止: すでに非常停止中なので何もしない");
            return;
        }
        Trace.Write(DiagnosticLevel.Error,
            $"非常停止: 電圧 {AppliedVolts:+0.00;-0.00;+0.00}→0 V, x={_state.X:+0.000;-0.000;+0.000} m, θ={CartPoleDynamics.WrapAngle(_state.Theta) * RadToDeg:+0.0;-0.0;+0.0}° (sim t={_simSeconds:0.000} s)");
        EmergencyStopped = true;
        DisableDrive(MotionEventKind.EmergencyStop, nowMs);
    }

    /// <summary>振子に外乱トルクを与える。direction が正なら +x 側へ倒す向き。</summary>
    public void Push(int direction)
    {
        _disturbance = new Disturbance(0.0, Math.Sign(direction) * DisturbanceTorque);
        _disturbanceEndSeconds = _simSeconds + DisturbanceSeconds;
        Trace.Write(DiagnosticLevel.Information,
            $"外乱: 軸トルク {_disturbance.Torque:+0.00;-0.00;+0.00} Nm を {DisturbanceSeconds * 1000.0:0} ms 印加 (sim t={_simSeconds:0.000} s, θ={CartPoleDynamics.WrapAngle(_state.Theta) * RadToDeg:+0.0;-0.0;+0.0}°)");
    }

    /// <summary>
    /// 壁時計 <paramref name="nowMs"/> に追いつくまで 1ms ステップで積分する。
    /// 積分中に帰還・イベントの送出タイミングが来たら、送信処理の時刻ではなく
    /// そのステップに対応する時刻を計測時刻として付けて発火する。
    /// </summary>
    public void AdvanceTo(double nowMs)
    {
        if (!_originSet)
        {
            _originMs = nowMs;
            _originSet = true;
            Trace.Write(DiagnosticLevel.Information,
                $"積分開始: 壁時計 {nowMs:0.0} ms を sim t=0 の原点にする (固定ステップ {StepSeconds * 1000.0:0} ms, 帰還周期 {_network.PeriodMs} ms, 追いつき上限 {MaxCatchUpSeconds * 1000.0:0} ms)");
        }

        double target = (nowMs - _originMs) / 1000.0;
        if (target - _simSeconds > MaxCatchUpSeconds)
        {
            // 長時間止まっていた (タブ非アクティブなど)。実時間に追いつこうとせず時間を捨てる。
            TimeSlips++;
            double droppedMs = (target - _simSeconds - MaxCatchUpSeconds) * 1000.0;
            _originMs = nowMs - (_simSeconds + MaxCatchUpSeconds) * 1000.0;
            target = _simSeconds + MaxCatchUpSeconds;
            Trace.Write(DiagnosticLevel.Warning,
                $"時間スリップ #{TimeSlips}: 実時間に {droppedMs + MaxCatchUpSeconds * 1000.0:0} ms 遅れていたので {droppedMs:0} ms を捨て、原点を後ろへずらした (sim t={_simSeconds:0.000} s)");
        }

        int stepsThisCall = 0;
        _advanceCallsSinceSummary++;
        _maxLagMsSinceSummary = Math.Max(_maxLagMsSinceSummary, (target - _simSeconds) * 1000.0);

        while (_simSeconds + StepSeconds <= target + 1e-9)
        {
            if (_disturbanceEndSeconds >= 0.0 && _simSeconds >= _disturbanceEndSeconds)
            {
                Trace.Write(DiagnosticLevel.Debug, $"外乱終了 (sim t={_simSeconds:0.000} s)");
                _disturbance = Disturbance.None;
                _disturbanceEndSeconds = -1.0;
            }

            _state = CartPoleDynamics.Rk4Step(
                _parameters, _derived, _state, DriveEnabled ? _volts : 0.0, _disturbance, StepSeconds);
            _simSeconds += StepSeconds;
            StepCount++;
            stepsThisCall++;
            _maxAbsXDotSinceSummary = Math.Max(_maxAbsXDotSinceSummary, Math.Abs(_state.XDot));

            double stepWallMs = _originMs + _simSeconds * 1000.0;

            if (DriveEnabled && Math.Abs(_state.X) > LimitTriggerPosition)
            {
                Trace.Write(DiagnosticLevel.Error,
                    $"メカリミット到達: |x|={Math.Abs(_state.X):0.000} m > {LimitTriggerPosition:0.000} m, ẋ={_state.XDot:+0.00;-0.00;+0.00} m/s, 電圧 {_volts:+0.00;-0.00;+0.00} V → ドライブ遮断");
                DisableDrive(MotionEventKind.LimitReached, stepWallMs);
            }

            if (_commandRunning && !_timeoutFired && stepWallMs - _lastCommandAtMs > CommandTimeoutMs)
            {
                _timeoutFired = true;
                Trace.Write(DiagnosticLevel.Warning,
                    $"指令タイムアウト: 最後の指令 (#{_activeCommandId}/{_lastSeq}) から {stepWallMs - _lastCommandAtMs:0} ms > {CommandTimeoutMs:0} ms → 電圧 {_volts:+0.00;-0.00;+0.00}→0 V");
                _volts = 0.0;
                _commandRunning = false;
                RaiseMotionEvent(MotionEventKind.CommandTimeout, stepWallMs);
            }

            if (_simSeconds * 1000.0 >= _nextFeedbackMs - 1e-6)
            {
                var (cart, pendulum) = ReadEncoders();
                if (Trace.IsEnabled(DiagnosticLevel.Trace))
                {
                    Trace.Write(DiagnosticLevel.Trace,
                        $"帰還 Seq {_feedbackSeq} t={stepWallMs:0.0} cart={cart} pend={pendulum} " +
                        $"(x={_state.X:+0.0000;-0.0000;+0.0000} θ={CartPoleDynamics.WrapAngle(_state.Theta) * RadToDeg:+0.00;-0.00;+0.00}° u={AppliedVolts:+0.00;-0.00;+0.00}) " +
                        $"cmd=#{_activeCommandId} e2e={MeasuredE2EMs:0.0} drive={(DriveEnabled ? "ON" : "OFF")} epoch={Epoch}");
                }
                FeedbackReady?.Invoke(new EncoderFeedback(
                    _feedbackSeq++, _activeCommandId, stepWallMs, cart, pendulum,
                    MeasuredE2EMs, DriveEnabled, Epoch));

                _nextFeedbackMs += _network.PeriodMs;
                if (_nextFeedbackMs < _simSeconds * 1000.0) _nextFeedbackMs = _simSeconds * 1000.0 + _network.PeriodMs;
            }

            _maxStepsPerAdvance = Math.Max(_maxStepsPerAdvance, stepsThisCall);
            if (_simSeconds >= _nextSummaryAtSeconds) TraceSummary();
        }
    }

    private void DisableDrive(MotionEventKind kind, double nowMs)
    {
        DriveEnabled = false;
        _volts = 0.0;
        _commandRunning = false;
        Trace.Write(DiagnosticLevel.Warning, $"ドライブ遮断 ({kind}): 以後の指令はすべて拒否する");
        RaiseMotionEvent(kind, nowMs);
    }

    private void RaiseMotionEvent(MotionEventKind kind, double nowMs)
    {
        var (cart, pendulum) = ReadEncoders();
        Trace.Write(DiagnosticLevel.Information,
            $"MotionEvent {kind} を送出: CommandId #{_activeCommandId}, 計測時刻 {nowMs:0.0} ms, cart={cart} cnt, pend={pendulum} cnt");
        MotionEventRaised?.Invoke(new MotionEvent(kind, _activeCommandId, nowMs, cart, pendulum));
    }

    private void Reject(VoltageCommand command, string reason)
    {
        RejectedCommands++;
        _rejectedSinceLastApplied++;
        // 拒否は連続して起きる (旧セッションの指令が遅延線に残っている等)。最初の 1 件だけ Debug、以降は Trace。
        Trace.Write(_rejectedSinceLastApplied == 1 ? DiagnosticLevel.Debug : DiagnosticLevel.Trace,
            $"指令 #{command.CommandId}/{command.Seq} ({command.Volts:+0.00;-0.00;+0.00} V) を拒否: {reason} (累計 {RejectedCommands})");
    }

    private (long Cart, long Pendulum) ReadEncoders()
    {
        double noise = _network.EncoderNoiseCounts;
        return (
            (long)Math.Round(_state.X / _derived.MetersPerCount + Gaussian() * noise),
            (long)Math.Round(_state.Theta / (2.0 * Math.PI) * _parameters.PendulumEncoderCountsPerRev + Gaussian() * noise));
    }

    /// <summary>Box-Muller 法。エンコーダの読み取りノイズに使う。</summary>
    private double Gaussian()
        => Math.Sqrt(-2.0 * Math.Log(1.0 - _random.NextDouble())) * Math.Cos(2.0 * Math.PI * _random.NextDouble());

    /// <summary>
    /// シミュレーション時刻で 1 秒ごとに、その間の動きを 1 行にまとめて出す。
    /// 「積分が進んでいるか」「実時間に追従できているか」「指令が届いて適用されているか」をこの 1 行で判断できるようにする。
    /// </summary>
    private void TraceSummary()
    {
        if (Trace.IsEnabled(DiagnosticLevel.Debug))
        {
            Trace.Write(DiagnosticLevel.Debug,
                $"集計 sim t={_simSeconds:0.0}s: {StepCount - _stepsAtSummary} step / {_advanceCallsSinceSummary} 回の AdvanceTo (最大 {_maxStepsPerAdvance} step/回, 実時間との遅れ 最大 {_maxLagMsSinceSummary:0.0} ms), " +
                $"帰還 {_feedbackSeq - _feedbackAtSummary} 本, 指令 適用 {AppliedCommands - _appliedAtSummary}/拒否 {RejectedCommands - _rejectedAtSummary}, " +
                $"x={_state.X:+0.000;-0.000;+0.000} m ẋ={_state.XDot:+0.00;-0.00;+0.00} m/s (最大 |ẋ| {_maxAbsXDotSinceSummary:0.00}), θ={CartPoleDynamics.WrapAngle(_state.Theta) * RadToDeg:+0.0;-0.0;+0.0}° θ̇={_state.ThetaDot:+0.00;-0.00;+0.00} rad/s, " +
                $"u={AppliedVolts:+0.00;-0.00;+0.00} V (最大 |u| {_maxAbsVoltsSinceSummary:0.0}), E2E {MeasuredE2EMs:0.0} ms, cmd=#{_activeCommandId}, drive={(DriveEnabled ? "ON" : "OFF")}, epoch={Epoch}, スリップ {TimeSlips}");
        }

        _stepsAtSummary = StepCount;
        _feedbackAtSummary = _feedbackSeq;
        _appliedAtSummary = AppliedCommands;
        _rejectedAtSummary = RejectedCommands;
        _maxAbsVoltsSinceSummary = 0.0;
        _maxAbsXDotSinceSummary = 0.0;
        _advanceCallsSinceSummary = 0;
        _maxStepsPerAdvance = 0;
        _maxLagMsSinceSummary = 0.0;
        _nextSummaryAtSeconds = _simSeconds + SummaryIntervalSeconds;
    }
}
