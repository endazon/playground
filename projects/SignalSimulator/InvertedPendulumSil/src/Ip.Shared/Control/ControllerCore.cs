using Ip.Shared.Model;
using Ip.Shared.Numerics;
using Ip.Shared.Protocol;

namespace Ip.Shared.Control;

/// <summary>離散化した設計モデルと、そこから得たゲイン・遅延余裕。</summary>
/// <param name="Ad">離散 A 行列 (予測器が使う)</param>
/// <param name="Bd">離散 B ベクトル</param>
/// <param name="Info">UI へ配信する設計結果</param>
public readonly record struct ControlDesign(double[,] Ad, double[] Bd, DesignInfo Info);

/// <summary>
/// バックエンドに置かれるコントローラ本体。純 SIL 構成なので、操作量 (電圧) を決めるのはここだけである。
///
/// 設計方針:
/// <list type="bullet">
///   <item>時刻は必ず引数で受け取る。実時間・仮想時間のどちらでも同じコードを動かせるようにするため。</item>
///   <item>送信は直接行わずイベントで外に出す。SignalR・遅延線・テストハーネスを差し替えられるようにするため。</item>
///   <item>「自分の指令が実行された前提」で状態を進めない。状態は必ず帰還から作る。</item>
/// </list>
///
/// スレッド安全ではない。呼び出し側が単一のループに直列化すること
/// (<c>Ip.Server</c> では接続ごとの Channel で直列化している)。
/// </summary>
public sealed class ControllerCore
{
    /// <summary>倒立とみなす角度 [rad]。これを超えたら FAULT。</summary>
    public const double FallAngleRad = 0.6;
    /// <summary>スイングアップ完了とみなす角度 [rad]</summary>
    public const double CaptureAngleRad = 0.3;
    /// <summary>スイングアップ完了とみなす角速度 [rad/s]</summary>
    public const double CaptureRateRadPerSec = 4.0;
    /// <summary>レール端から手前に取るソフトリミットの余裕 [m]</summary>
    public const double SoftLimitMarginM = 0.05;
    /// <summary>台車目標位置のランプ速度 [m/s]</summary>
    public const double TargetRampMetersPerSec = 0.3;

    private static readonly double[] StateWeights = [40.0, 60.0, 1.0, 0.5];
    private const double InputWeight = 0.6;
    private const int HistoryCapacity = 128;

    private readonly SwingUpGains _swingUpGains;
    private readonly List<double> _voltageHistory = new(HistoryCapacity);

    private PendulumParameters _parameters;
    private DerivedParameters _derived;
    private NetworkConfig _network = new();
    private ControlOptions _options = new();

    private double[,] _ad = new double[4, 4];
    private double[] _bd = new double[4];
    private double[] _gains = [0, 0, 0, 0];

    private readonly VelocityEstimator _cartVelocity;
    private readonly VelocityEstimator _pendulumVelocity;
    private bool _estimatorReady;
    private double _estX, _estTheta, _estTimestampMs;
    private long _estSeq = -1;
    private int _estEpoch = -1;

    private long _seq;
    private double _volts;
    private double _targetX;
    private double _startLocalMs;
    private double _lastFeedbackLocalMs;
    private double _feedbackIntervalMs;
    private double _uplinkDelayMs;
    private double _e2eDelayMs;
    private long _feedbackCount;
    private long _staleEventCount;

    public ControllerCore(PendulumParameters? designParameters = null, SwingUpGains? swingUpGains = null)
    {
        _parameters = designParameters ?? new PendulumParameters();
        _derived = _parameters.Derive();
        _swingUpGains = swingUpGains ?? new SwingUpGains();
        _cartVelocity = new VelocityEstimator(_options.VelocityCutoffHz);
        _pendulumVelocity = new VelocityEstimator(_options.VelocityCutoffHz);
        Redesign();
    }

    /// <summary>電圧指令を送るべきタイミングで発火する。</summary>
    public event Action<VoltageCommand>? VoltageProduced;
    /// <summary>セッション打ち切りを送るべきタイミングで発火する。</summary>
    public event Action<AbortCommand>? AbortProduced;
    /// <summary>設計をやり直したときに発火する。</summary>
    public event Action<DesignInfo>? DesignUpdated;
    /// <summary>オペレータに見せるログ。</summary>
    public event Action<LogEntry>? Logged;

    public ControlMode Mode { get; private set; } = ControlMode.Idle;
    public string Reason { get; private set; } = string.Empty;
    public long CommandId { get; private set; }
    public DesignInfo Design { get; private set; } = null!;
    public NetworkConfig Network => _network;
    public ControlOptions Options => _options;
    public PendulumParameters Parameters => _parameters;

    /// <summary>
    /// プラント時刻 → コントローラ時刻のオフセット [ms] (local = plant + offset)。
    /// 別ホストで動く以上、時計は一致しない。<see cref="ClockSynchronizer"/> が推定した値を入れる。
    /// </summary>
    public double ClockOffsetMs { get; set; }

    public bool IsRunning => Mode is ControlMode.SwingUp or ControlMode.Balance;

    public void SetNetwork(NetworkConfig config)
    {
        config = config.Sanitized();
        bool periodChanged = config.PeriodMs != _network.PeriodMs;
        _network = config;
        if (periodChanged) Redesign();
    }

    public void SetOptions(ControlOptions options)
    {
        options = options.Sanitized();
        bool cutoffChanged = Math.Abs(options.VelocityCutoffHz - _options.VelocityCutoffHz) > 1e-9;
        _options = options;
        _cartVelocity.CutoffHz = options.VelocityCutoffHz;
        _pendulumVelocity.CutoffHz = options.VelocityCutoffHz;
        if (cutoffChanged) Redesign(); // 速度推定の帯域は遅延余裕に効くので再評価する
    }

    public void SetPlant(PlantConfig config)
    {
        config = config.Sanitized();
        if (Math.Abs(config.PendulumLength - _parameters.PendulumLength) < 1e-12) return;
        _parameters = _parameters with { PendulumLength = config.PendulumLength };
        _derived = _parameters.Derive();
        Redesign();
    }

    public void Operate(OperatorAction action, double nowMs)
    {
        switch (action)
        {
            case OperatorAction.SwingUp:
                Start(ControlMode.SwingUp, nowMs);
                break;
            case OperatorAction.Balance:
                Start(ControlMode.Balance, nowMs);
                break;
            case OperatorAction.Stop:
                if (Mode != ControlMode.Idle)
                {
                    Abort();
                    SetMode(ControlMode.Idle, "オペレータ停止");
                }
                break;
            case OperatorAction.ClearFault:
                if (Mode == ControlMode.Fault) SetMode(ControlMode.Idle, "異常リセット");
                break;
        }
    }

    /// <summary>定周期の帰還を受け取り、そのまま制御計算まで行う。</summary>
    public void OnFeedback(EncoderFeedback feedback, double nowMs)
    {
        if (feedback.Seq <= _estSeq) return;            // 逆行した帰還は無視 (遅延線の追い越しは無いが念のため)
        _lastFeedbackLocalMs = nowMs;

        double measuredLocalMs = feedback.TimestampMs + ClockOffsetMs;
        if (measuredLocalMs < _startLocalMs) return;    // 開始指示より前の計測は使わない

        if (feedback.Epoch != _estEpoch)
        {
            // プラントがリセットされた: 古い計測との差分で速度を作らないよう推定器を初期化する
            _estEpoch = feedback.Epoch;
            _estimatorReady = false;
        }

        double x = feedback.CartCounts * _derived.MetersPerCount;
        double theta = feedback.PendulumCounts * 2.0 * Math.PI / _parameters.PendulumEncoderCountsPerRev;

        if (!_estimatorReady)
        {
            _estimatorReady = true;
            _estX = x;
            _estTheta = theta;
            _cartVelocity.Reset(x);
            _pendulumVelocity.Reset(theta);
        }
        else
        {
            double dt = (feedback.TimestampMs - _estTimestampMs) / 1000.0;
            if (dt > 0.0)
            {
                _cartVelocity.Update(x, dt);
                _pendulumVelocity.Update(theta, dt);
                _feedbackIntervalMs = Ema(_feedbackIntervalMs, feedback.TimestampMs - _estTimestampMs);
            }
            _estX = x;
            _estTheta = theta;
        }

        _estTimestampMs = feedback.TimestampMs;
        _estSeq = feedback.Seq;
        _feedbackCount++;
        _uplinkDelayMs = Ema(_uplinkDelayMs, nowMs - measuredLocalMs);
        if (feedback.E2EMs > 0.0) _e2eDelayMs = Ema(_e2eDelayMs, feedback.E2EMs, 0.05);

        Control(feedback.TimestampMs);
    }

    /// <summary>プラント側の保護動作を受け取る。運転中なら FAULT に落とす。</summary>
    public void OnMotionEvent(MotionEvent motionEvent, double nowMs)
    {
        _ = nowMs;
        if (motionEvent.CommandId < CommandId)
        {
            // リセット直後に届いた旧セッションのイベントで転倒判定しないための世代チェック
            _staleEventCount++;
            Log(Protocol.LogSeverity.Warning, $"旧セッション(#{motionEvent.CommandId})のイベント {motionEvent.Kind} を破棄");
            return;
        }

        if (IsRunning) Fault($"プラント通知: {Describe(motionEvent.Kind)}");
        else Log(Protocol.LogSeverity.Warning, $"プラント通知: {Describe(motionEvent.Kind)}");
    }

    /// <summary>ウォッチドッグ。帰還が途絶えたことを検知する。制御周期より短い間隔で呼ぶこと。</summary>
    public void Tick(double nowMs)
    {
        double watchdogMs = Math.Max(_options.WatchdogMs, 3.0 * _network.PeriodMs);
        if (IsRunning && nowMs - _lastFeedbackLocalMs > watchdogMs)
            Fault($"ウォッチドッグ: {watchdogMs:0} ms 帰還なし");
    }

    public ControllerStatus Snapshot(int downlinkQueueLength = 0) => new(
        Mode, Reason, CommandId, _volts, _targetX,
        _estX, CartPoleDynamics.WrapAngle(_estTheta),
        _feedbackIntervalMs, _uplinkDelayMs, _e2eDelayMs,
        _feedbackCount, _staleEventCount, downlinkQueueLength);

    // ---- 内部 ----

    private static double Ema(double old, double sample, double alpha = 0.1)
        => old == 0.0 ? sample : old + alpha * (sample - old);

    private static string Describe(MotionEventKind kind) => kind switch
    {
        MotionEventKind.EmergencyStop => "非常停止",
        MotionEventKind.LimitReached => "メカリミット到達",
        MotionEventKind.CommandTimeout => "指令タイムアウト",
        MotionEventKind.DriveDisabled => "ドライブ無効中の指令拒否",
        _ => kind.ToString(),
    };

    /// <summary>
    /// 帰還周期と振子長から離散 LQR ゲインと理論遅延余裕を設計する。
    /// コントローラ本体とテスト・解析ツールが同じ経路を通るように公開している。
    /// </summary>
    public static ControlDesign CreateDesign(PendulumParameters parameters, int periodMs, double velocityCutoffHz)
    {
        var model = LinearizedModel.Create(parameters);
        double ts = periodMs / 1000.0;
        var (ad, bd) = Discretization.ZeroOrderHold(model.A, model.B, ts);
        var gains = DiscreteLqr.Solve(ad, bd, StateWeights, InputWeight);
        int margin = DelayMarginAnalyzer.Compute(model.A, model.B, gains, ts, velocityCutoffHz);
        return new ControlDesign(ad, bd, new DesignInfo(gains, model.UnstablePole, margin, periodMs));
    }

    private void Redesign()
    {
        var design = CreateDesign(_parameters, _network.PeriodMs, _options.VelocityCutoffHz);
        _ad = design.Ad;
        _bd = design.Bd;
        _gains = design.Info.Gains;
        Design = design.Info;
        DesignUpdated?.Invoke(Design);
    }

    private void Start(ControlMode target, double nowMs)
    {
        if (Mode != ControlMode.Idle)
        {
            Log(Protocol.LogSeverity.Warning,
                $"{Mode} 中は開始できません{(Mode == ControlMode.Fault ? " (異常リセットが必要)" : "")}");
            return;
        }

        CommandId++;
        _seq = 0;
        _voltageHistory.Clear();
        _startLocalMs = nowMs;
        _lastFeedbackLocalMs = nowMs;
        _e2eDelayMs = 0.0;
        _estimatorReady = false;
        _estSeq = -1;
        _targetX = 0.0;
        SetMode(target, $"CommandId #{CommandId}");
    }

    private void SetMode(ControlMode mode, string reason)
    {
        if (mode == Mode) return;
        var previous = Mode;
        Mode = mode;
        Reason = reason;
        Log(mode == ControlMode.Fault ? Protocol.LogSeverity.Error : Protocol.LogSeverity.Info,
            $"{previous} → {mode}{(string.IsNullOrEmpty(reason) ? "" : $"  ({reason})")}");
    }

    private void Send(double volts, double basedOnMs)
    {
        volts = CartPoleDynamics.Clamp(volts, -_parameters.MaxVoltage, _parameters.MaxVoltage);
        _volts = volts;
        _voltageHistory.Add(volts);
        if (_voltageHistory.Count > HistoryCapacity) _voltageHistory.RemoveAt(0);
        VoltageProduced?.Invoke(new VoltageCommand(CommandId, _seq++, volts, basedOnMs));
    }

    private void Abort()
    {
        _volts = 0.0;
        AbortProduced?.Invoke(new AbortCommand(CommandId));
    }

    private void Fault(string reason)
    {
        if (Mode == ControlMode.Fault) return;
        Abort();
        SetMode(ControlMode.Fault, reason);
    }

    private void Control(double measuredAtMs)
    {
        if (!IsRunning) return;
        double theta = CartPoleDynamics.WrapAngle(_estTheta);

        if (Mode == ControlMode.SwingUp)
        {
            if (Math.Abs(theta) < CaptureAngleRad && Math.Abs(_pendulumVelocity.Value) < CaptureRateRadPerSec)
            {
                SetMode(ControlMode.Balance, "倒立領域に到達");
            }
            else
            {
                if (Math.Abs(_estX) > _parameters.RailStroke - SoftLimitMarginM)
                {
                    Fault("ソフトリミット超過");
                    return;
                }
                var state = new PlantState(_estX, _estTheta, _cartVelocity.Value, _pendulumVelocity.Value);
                Send(SwingUpController.Voltage(_parameters, _derived, state, _swingUpGains), measuredAtMs);
                return;
            }
        }

        if (Math.Abs(theta) > FallAngleRad)
        {
            Fault($"転倒検知 |θ|={Math.Abs(theta) * 180.0 / Math.PI:0}°");
            return;
        }
        if (Math.Abs(_estX) > _parameters.RailStroke - SoftLimitMarginM)
        {
            Fault("ソフトリミット超過");
            return;
        }

        double rampStep = TargetRampMetersPerSec * _network.PeriodMs / 1000.0;
        _targetX += CartPoleDynamics.Clamp(_options.CartTargetMeters - _targetX, -rampStep, rampStep);

        double[] x = [_estX - _targetX, theta, _cartVelocity.Value, _pendulumVelocity.Value];
        if (_options.UsePredictor)
            x = Predict(x, (int)Math.Round(_e2eDelayMs / _network.PeriodMs));

        double u = -(_gains[0] * x[0] + _gains[1] * x[1] + _gains[2] * x[2] + _gains[3] * x[3]);
        Send(u, measuredAtMs);
    }

    /// <summary>
    /// 送信済みの電圧で N ステップ先の状態を予測する (簡易 Smith 型補償)。
    /// 「まだ結果が返ってきていないが、すでに送った指令」の効果だけを離散モデルで進める。
    /// モデル誤差とジッタがそのまま予測誤差になるので万能ではない。
    /// </summary>
    private double[] Predict(double[] x, int steps)
    {
        steps = Math.Min(Math.Max(steps, 0), _voltageHistory.Count);
        for (int j = steps; j >= 1; j--)
        {
            var next = Matrix.Multiply(_ad, x);
            double u = _voltageHistory[^j];
            for (int i = 0; i < 4; i++) next[i] += _bd[i] * u;
            x = next;
        }
        return x;
    }

    private void Log(Protocol.LogSeverity level, string message)
        => Logged?.Invoke(new LogEntry(level, message, Mode, CommandId));
}
