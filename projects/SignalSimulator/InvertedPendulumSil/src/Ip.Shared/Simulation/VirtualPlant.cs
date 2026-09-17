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
/// </summary>
public sealed class VirtualPlant
{
    /// <summary>固定積分ステップ [s]</summary>
    public const double StepSeconds = 0.001;
    /// <summary>指令が途絶えたときに電圧を切るまでの時間 [ms]</summary>
    public const double CommandTimeoutMs = 500.0;
    /// <summary>1 回の <see cref="AdvanceTo"/> で許す最大の追いつき時間 [s]。これを超えたぶんは捨てる。</summary>
    public const double MaxCatchUpSeconds = 0.2;
    /// <summary>外乱ボタンで与える軸トルク [Nm] とその継続時間 [s]</summary>
    private const double DisturbanceTorque = 0.3;
    private const double DisturbanceSeconds = 0.05;

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

    public PlantState State => _state;
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

    public void Configure(NetworkConfig config)
    {
        config = config.Sanitized();
        bool periodChanged = config.PeriodMs != _network.PeriodMs;
        _network = config;
        if (periodChanged) _nextFeedbackMs = _simSeconds * 1000.0; // 次のステップで 1 回送ってから新周期に乗る
    }

    public void Configure(PlantConfig config)
    {
        config = config.Sanitized();
        if (Math.Abs(config.PendulumLength - _parameters.PendulumLength) < 1e-12) return;
        _parameters = _parameters with { PendulumLength = config.PendulumLength };
        _derived = _parameters.Derive();
    }

    /// <summary>
    /// 下りの電圧指令を適用する。実機のドライブが行う受け入れ判定をここで再現している。
    /// </summary>
    public void ApplyCommand(VoltageCommand command, double nowMs)
    {
        if (command.CommandId < _activeCommandId)
        {
            RejectedCommands++;                       // 旧セッションの指令
            return;
        }

        if (command.CommandId > _activeCommandId)
        {
            // 新しい運転セッション。ドライブが無効のままならセッションごと拒否して通知する
            _activeCommandId = command.CommandId;
            _lastSeq = -1;
            _timeoutFired = false;
            _sessionAccepted = DriveEnabled;
            if (!DriveEnabled) RaiseMotionEvent(MotionEventKind.DriveDisabled, nowMs);
        }

        if (!_sessionAccepted || !DriveEnabled || command.Seq <= _lastSeq)
        {
            RejectedCommands++;
            return;
        }

        _lastSeq = command.Seq;
        _volts = CartPoleDynamics.Clamp(command.Volts, -_parameters.MaxVoltage, _parameters.MaxVoltage);
        _lastCommandAtMs = nowMs;
        MeasuredE2EMs = nowMs - command.BasedOnMs;    // 同じプラント時計の差なので時計ズレの影響を受けない
        _commandRunning = true;
        AppliedCommands++;
    }

    public void ApplyAbort(AbortCommand command)
    {
        if (command.CommandId < _activeCommandId) return;
        _activeCommandId = command.CommandId;
        _volts = 0.0;
        _commandRunning = false;
    }

    /// <summary>現場側の復帰操作。ドライブを再び有効にして状態を初期姿勢へ戻す。</summary>
    public void Reset(bool upright)
    {
        _state = upright ? PlantState.NearUpright() : PlantState.Hanging;
        _volts = 0.0;
        _commandRunning = false;
        DriveEnabled = true;
        EmergencyStopped = false;
        _sessionAccepted = true;
        _timeoutFired = false;
        _disturbance = Disturbance.None;
        _disturbanceEndSeconds = -1.0;
        Epoch++;
    }

    public void EmergencyStop(double nowMs)
    {
        if (!DriveEnabled && EmergencyStopped) return;
        EmergencyStopped = true;
        DisableDrive(MotionEventKind.EmergencyStop, nowMs);
    }

    /// <summary>振子に外乱トルクを与える。direction が正なら +x 側へ倒す向き。</summary>
    public void Push(int direction)
    {
        _disturbance = new Disturbance(0.0, Math.Sign(direction) * DisturbanceTorque);
        _disturbanceEndSeconds = _simSeconds + DisturbanceSeconds;
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
        }

        double target = (nowMs - _originMs) / 1000.0;
        if (target - _simSeconds > MaxCatchUpSeconds)
        {
            // 長時間止まっていた (タブ非アクティブなど)。実時間に追いつこうとせず時間を捨てる。
            TimeSlips++;
            _originMs = nowMs - (_simSeconds + MaxCatchUpSeconds) * 1000.0;
            target = _simSeconds + MaxCatchUpSeconds;
        }

        while (_simSeconds + StepSeconds <= target + 1e-9)
        {
            if (_disturbanceEndSeconds >= 0.0 && _simSeconds >= _disturbanceEndSeconds)
            {
                _disturbance = Disturbance.None;
                _disturbanceEndSeconds = -1.0;
            }

            _state = CartPoleDynamics.Rk4Step(
                _parameters, _derived, _state, DriveEnabled ? _volts : 0.0, _disturbance, StepSeconds);
            _simSeconds += StepSeconds;

            double stepWallMs = _originMs + _simSeconds * 1000.0;

            if (DriveEnabled && Math.Abs(_state.X) > _parameters.RailStroke + 0.002)
                DisableDrive(MotionEventKind.LimitReached, stepWallMs);

            if (_commandRunning && !_timeoutFired && stepWallMs - _lastCommandAtMs > CommandTimeoutMs)
            {
                _timeoutFired = true;
                _volts = 0.0;
                _commandRunning = false;
                RaiseMotionEvent(MotionEventKind.CommandTimeout, stepWallMs);
            }

            if (_simSeconds * 1000.0 >= _nextFeedbackMs - 1e-6)
            {
                var (cart, pendulum) = ReadEncoders();
                FeedbackReady?.Invoke(new EncoderFeedback(
                    _feedbackSeq++, _activeCommandId, stepWallMs, cart, pendulum,
                    MeasuredE2EMs, DriveEnabled, Epoch));

                _nextFeedbackMs += _network.PeriodMs;
                if (_nextFeedbackMs < _simSeconds * 1000.0) _nextFeedbackMs = _simSeconds * 1000.0 + _network.PeriodMs;
            }
        }
    }

    private void DisableDrive(MotionEventKind kind, double nowMs)
    {
        DriveEnabled = false;
        _volts = 0.0;
        _commandRunning = false;
        RaiseMotionEvent(kind, nowMs);
    }

    private void RaiseMotionEvent(MotionEventKind kind, double nowMs)
    {
        var (cart, pendulum) = ReadEncoders();
        MotionEventRaised?.Invoke(new MotionEvent(kind, _activeCommandId, nowMs, cart, pendulum));
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
}
