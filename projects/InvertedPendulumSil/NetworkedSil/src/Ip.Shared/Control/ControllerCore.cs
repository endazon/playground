using System.Collections.Concurrent;
using Ip.Shared.Diagnostics;
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
///
/// 動作の可視化のため、オペレータ向けの <see cref="Logged"/> とは別に
/// 診断トレース <see cref="Trace"/> を持つ。帰還 1 本ごとの内容は Trace、
/// 1 秒ごとの集計は Debug、節目 (状態遷移・設定変更・再設計) は Information で出す。
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
    public const double SoftLimitMarginM = CartLimits.SoftLimitMarginM;

    /// <summary>
    /// 予測器が外挿してよい最大時間 [ms]。
    /// 実測 E2E 遅延は通信スパイクや時間スリップで跳ねることがあり、その値をそのまま使うと
    /// 不安定モデルで何百 ms も開ループ外挿して、飽和寸前の電圧を叩き出す。
    /// </summary>
    public const double MaxPredictionHorizonMs = 300.0;
    /// <summary>台車目標位置のランプ速度 [m/s]</summary>
    public const double TargetRampMetersPerSec = 0.3;

    /// <summary>診断トレースの集計を出す間隔 (プラント時計) [ms]</summary>
    public const double SummaryIntervalMs = 1000.0;

    private static readonly double[] StateWeights = [40.0, 60.0, 1.0, 0.5];
    private const double InputWeight = 0.6;
    private const int HistoryCapacity = 128;
    private const double RadToDeg = 180.0 / Math.PI;

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
    private readonly ExponentialAverage _feedbackInterval = new(0.1);
    private readonly ExponentialAverage _uplinkDelay = new(0.1);
    private readonly ExponentialAverage _e2eDelay = new(0.05);
    private bool _estimatorReady;
    private double _estX, _estTheta, _estTimestampMs;
    private long _estSeq = -1;
    private int _estEpoch = -1;

    private long _seq;
    private double _volts;
    private double _targetX;
    private double _lastFeedbackLocalMs;
    /// <summary>直近に受け取った帰還の計測時刻 (プラント時計)。開始時刻ガードの基準に使う。</summary>
    private double _lastPlantTimestampMs;
    /// <summary>開始操作を出した時点のプラント時計。これより前に計測された帰還は使わない。</summary>
    private double _startPlantMs;
    private long _feedbackCount;
    private long _staleEventCount;

    // ---- 診断トレース用の集計 (制御には使わない) ----
    private long _droppedBackwardCount;
    private long _droppedBeforeStartCount;
    private long _saturatedCount;
    private long _feedbackCountAtSummary;
    private long _seqAtSummary;
    private long _saturatedAtSummary;
    private double _summaryDueAtPlantMs = double.NaN;
    private double _maxAbsThetaSinceSummary;
    private double _maxAbsVoltsSinceSummary;
    private double _maxDtSinceSummary;
    private int _lastPredictSteps;
    private bool _silenceWarned;
    private bool _firstFeedbackTraced;

    public ControllerCore(PendulumParameters? designParameters = null, SwingUpGains? swingUpGains = null)
    {
        _parameters = designParameters ?? new PendulumParameters();
        _derived = _parameters.Derive();
        _swingUpGains = swingUpGains ?? new SwingUpGains();
        _cartVelocity = new VelocityEstimator(_options.VelocityCutoffHz);
        _pendulumVelocity = new VelocityEstimator(_options.VelocityCutoffHz);
        Redesign("初期化");
    }

    /// <summary>電圧指令を送るべきタイミングで発火する。</summary>
    public event Action<VoltageCommand>? VoltageProduced;
    /// <summary>セッション打ち切りを送るべきタイミングで発火する。</summary>
    public event Action<AbortCommand>? AbortProduced;
    /// <summary>設計をやり直したときに発火する。</summary>
    public event Action<DesignInfo>? DesignUpdated;
    /// <summary>オペレータに見せるログ。</summary>
    public event Action<LogEntry>? Logged;

    /// <summary>診断トレース。ホスト側でログ基盤へつなぐ。購読者がいなければ何も整形しない。</summary>
    public DiagnosticTrace Trace { get; } = new("controller");

    public ControlMode Mode { get; private set; } = ControlMode.Idle;
    public string Reason { get; private set; } = string.Empty;
    public long CommandId { get; private set; }
    public DesignInfo Design { get; private set; } = null!;
    public NetworkConfig Network => _network;
    public ControlOptions Options => _options;
    public PendulumParameters Parameters => _parameters;

    /// <summary>逆行した帰還 (Seq が進んでいない) を捨てた回数。</summary>
    public long DroppedBackwardFeedbackCount => _droppedBackwardCount;
    /// <summary>開始指示より前に計測された帰還を捨てた回数。</summary>
    public long DroppedBeforeStartFeedbackCount => _droppedBeforeStartCount;
    /// <summary>電圧指令が飽和 (±MaxVoltage) した回数。</summary>
    public long SaturatedCommandCount => _saturatedCount;

    /// <summary>
    /// プラント時刻 → コントローラ時刻のオフセット [ms] (local = plant + offset)。
    /// 別ホストで動く以上、時計は一致しない。<see cref="ClockSynchronizer"/> が推定した値を入れる。
    ///
    /// これは <b>「上り遅延」の表示にしか使わない</b>。制御の判断 (開始時刻ガード・速度推定・E2E 遅延) は
    /// すべてプラント時計だけで閉じており、この推定が外れても制御は成立する。
    /// </summary>
    public double ClockOffsetMs { get; set; }

    public bool IsRunning => Mode is ControlMode.SwingUp or ControlMode.Balance;

    public void SetNetwork(NetworkConfig config)
    {
        config = config.Sanitized();
        bool periodChanged = config.PeriodMs != _network.PeriodMs;
        var previous = _network;
        _network = config;

        Trace.Write(config == previous ? DiagnosticLevel.Debug : DiagnosticLevel.Information,
            $"通信条件: 周期 {previous.PeriodMs}→{config.PeriodMs} ms, 片道遅延 {config.LatencyMs:0} ms, ジッタ {config.JitterMs:0} ms, " +
            $"スパイク {config.SpikePercent:0.0} %, ノイズ σ{config.EncoderNoiseCounts:0.0} count" +
            $"{(config == previous ? " (変更なし)" : periodChanged ? " → 周期が変わったので再設計" : "")}");

        if (periodChanged) Redesign($"帰還周期 {previous.PeriodMs}→{config.PeriodMs} ms");
    }

    public void SetOptions(ControlOptions options)
    {
        options = options.Sanitized();
        bool cutoffChanged = Math.Abs(options.VelocityCutoffHz - _options.VelocityCutoffHz) > 1e-9;
        var previous = _options;
        _options = options;
        _cartVelocity.CutoffHz = options.VelocityCutoffHz;
        _pendulumVelocity.CutoffHz = options.VelocityCutoffHz;

        Trace.Write(options == previous ? DiagnosticLevel.Debug : DiagnosticLevel.Information,
            $"制御オプション: 予測器 {(previous.UsePredictor ? "ON" : "OFF")}→{(options.UsePredictor ? "ON" : "OFF")}, " +
            $"台車目標 {previous.CartTargetMeters:+0.00;-0.00;+0.00}→{options.CartTargetMeters:+0.00;-0.00;+0.00} m, " +
            $"速度LPF {options.VelocityCutoffHz:0.#} Hz, ウォッチドッグ {options.WatchdogMs:0} ms" +
            $"{(options == previous ? " (変更なし)" : cutoffChanged ? " → LPF 帯域が変わったので再設計" : "")}");

        if (cutoffChanged) Redesign($"速度LPF {previous.VelocityCutoffHz:0.#}→{options.VelocityCutoffHz:0.#} Hz"); // 速度推定の帯域は遅延余裕に効くので再評価する
    }

    public void SetPlant(PlantConfig config)
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
            $"プラント設定: 振子長 {previous:0.00}→{config.PendulumLength:0.00} m → 設計モデルを更新して再設計");
        Redesign($"振子長 {previous:0.00}→{config.PendulumLength:0.00} m");
    }

    public void Operate(OperatorAction action, double nowMs)
    {
        Trace.Write(DiagnosticLevel.Information,
            $"オペレータ操作 {action} を受信 (現在 {Mode}, CommandId #{CommandId}, t={nowMs:0} ms)");

        switch (action)
        {
            case OperatorAction.SwingUp:
                Start(ControlMode.SwingUp, nowMs);
                break;
            case OperatorAction.Balance:
                Start(ControlMode.Balance, nowMs);
                break;
            case OperatorAction.Stop:
                // FAULT からは抜けない。異常の解除は「異常リセット」だけの責務にする。
                // ここで Idle に落とすと、停止操作だけで異常がログにも残らず消えてしまう。
                if (IsRunning)
                {
                    Abort("オペレータ停止");
                    SetMode(ControlMode.Idle, "オペレータ停止");
                }
                else
                {
                    Trace.Write(DiagnosticLevel.Debug, $"停止操作: {Mode} 中なので何もしない");
                }
                break;
            case OperatorAction.ClearFault:
                if (Mode == ControlMode.Fault) SetMode(ControlMode.Idle, "異常リセット");
                else Trace.Write(DiagnosticLevel.Debug, $"異常リセット: {Mode} 中なので何もしない");
                break;
        }
    }

    /// <summary>定周期の帰還を受け取り、そのまま制御計算まで行う。</summary>
    public void OnFeedback(EncoderFeedback feedback, double nowMs)
    {
        if (feedback.Seq <= _estSeq)
        {
            // 逆行した帰還は無視 (遅延線の追い越しは無いが念のため)
            _droppedBackwardCount++;
            Trace.Write(DiagnosticLevel.Debug,
                $"帰還を破棄: Seq {feedback.Seq} が直近 {_estSeq} 以下 (累計 {_droppedBackwardCount})");
            return;
        }
        _lastPlantTimestampMs = feedback.TimestampMs;

        // 開始指示より前に計測された帰還は使わない。
        // 比較はプラント時計だけで閉じる。時計オフセットの推定に失敗しても、
        // 「全帰還を捨て続けるのにウォッチドッグは鳴らない」という静かな故障にはならない。
        if (feedback.TimestampMs < _startPlantMs)
        {
            _droppedBeforeStartCount++;
            Trace.Write(DiagnosticLevel.Debug,
                $"帰還を破棄: 計測時刻 {feedback.TimestampMs:0} ms が開始時刻 {_startPlantMs:0} ms より前 (累計 {_droppedBeforeStartCount})");
            return;
        }

        // ウォッチドッグの更新は、実際に使える帰還を受け取った後に行う。
        double silenceMs = nowMs - _lastFeedbackLocalMs;
        _lastFeedbackLocalMs = nowMs;
        if (_silenceWarned)
        {
            _silenceWarned = false;
            Trace.Write(DiagnosticLevel.Debug, $"帰還が復帰: {silenceMs:0} ms ぶりに Seq {feedback.Seq} を受信");
        }

        if (feedback.Epoch != _estEpoch)
        {
            // プラントがリセットされた: 古い計測との差分で速度を作らないよう推定器を初期化する
            Trace.Write(DiagnosticLevel.Information,
                $"プラントの Epoch が {_estEpoch}→{feedback.Epoch} に変わった → 速度推定器を初期化");
            _estEpoch = feedback.Epoch;
            _estimatorReady = false;
        }

        double x = feedback.CartCounts * _derived.MetersPerCount;
        double theta = feedback.PendulumCounts * 2.0 * Math.PI / _parameters.PendulumEncoderCountsPerRev;
        double dt = 0.0;

        if (!_estimatorReady)
        {
            _estimatorReady = true;
            _estX = x;
            _estTheta = theta;
            _cartVelocity.Reset(x);
            _pendulumVelocity.Reset(theta);
            Trace.Write(DiagnosticLevel.Information,
                $"推定器を初期化: Seq {feedback.Seq}, x={x:0.000} m, θ={CartPoleDynamics.WrapAngle(theta) * RadToDeg:0.0}°, 計測時刻 {feedback.TimestampMs:0} ms");
        }
        else
        {
            dt = (feedback.TimestampMs - _estTimestampMs) / 1000.0;
            if (dt > 0.0)
            {
                _cartVelocity.Update(x, dt);
                _pendulumVelocity.Update(theta, dt);
                _feedbackInterval.Update(feedback.TimestampMs - _estTimestampMs);
            }
            else
            {
                Trace.Write(DiagnosticLevel.Debug,
                    $"計測時刻が進んでいない帰還: Seq {feedback.Seq}, dt={dt * 1000.0:0.0} ms → 速度は更新しない");
            }
            _estX = x;
            _estTheta = theta;
        }

        _estTimestampMs = feedback.TimestampMs;
        _estSeq = feedback.Seq;
        _feedbackCount++;
        double uplinkMs = nowMs - (feedback.TimestampMs + ClockOffsetMs);
        _uplinkDelay.Update(uplinkMs);
        if (feedback.E2EMs > 0.0) _e2eDelay.Update(feedback.E2EMs);

        if (IsRunning && !_firstFeedbackTraced)
        {
            _firstFeedbackTraced = true;
            Trace.Write(DiagnosticLevel.Information,
                $"運転開始後の最初の帰還: Seq {feedback.Seq}, 開始から {feedback.TimestampMs - _startPlantMs:0} ms (プラント時計), 上り遅延 {uplinkMs:0.0} ms");
        }

        if (Trace.IsEnabled(DiagnosticLevel.Trace))
        {
            Trace.Write(DiagnosticLevel.Trace,
                $"帰還 Seq {feedback.Seq} t={feedback.TimestampMs:0.0} dt={dt * 1000.0:0.0}ms " +
                $"x={x:+0.0000;-0.0000;+0.0000} ({feedback.CartCounts} cnt) θ={CartPoleDynamics.WrapAngle(theta) * RadToDeg:+0.00;-0.00;+0.00}° ({feedback.PendulumCounts} cnt) " +
                $"ẋ={_cartVelocity.Value:+0.000;-0.000;+0.000} θ̇={_pendulumVelocity.Value:+0.000;-0.000;+0.000} " +
                $"up={uplinkMs:0.0}ms e2e={feedback.E2EMs:0.0}ms drive={(feedback.DriveEnabled ? "ON" : "OFF")} epoch={feedback.Epoch}");
        }

        // 冗長な安全信号: MotionEvent が遅れても、定周期帰還のフラグでドライブ遮断に気づける。
        if (IsRunning && !feedback.DriveEnabled)
        {
            Fault("プラント通知: ドライブ無効");
            return;
        }

        Control(feedback.TimestampMs);
        TraceSummaryIfDue(feedback.TimestampMs, dt);
    }

    /// <summary>プラント側の保護動作を受け取る。運転中なら FAULT に落とす。</summary>
    public void OnMotionEvent(MotionEvent motionEvent, double nowMs)
    {
        Trace.Write(DiagnosticLevel.Information,
            $"MotionEvent {motionEvent.Kind} を受信: CommandId #{motionEvent.CommandId} (現行 #{CommandId}), 計測時刻 {motionEvent.TimestampMs:0} ms, " +
            $"受信まで {nowMs - (motionEvent.TimestampMs + ClockOffsetMs):0} ms, cart={motionEvent.CartCounts} cnt, pend={motionEvent.PendulumCounts} cnt");

        if (motionEvent.CommandId < CommandId)
        {
            // リセット直後に届いた旧セッションのイベントで転倒判定しないための世代チェック
            _staleEventCount++;
            Trace.Write(DiagnosticLevel.Warning,
                $"旧セッション #{motionEvent.CommandId} のイベント {motionEvent.Kind} を破棄 (累計 {_staleEventCount})");
            Log(Protocol.LogSeverity.Warning, $"旧セッション(#{motionEvent.CommandId})のイベント {motionEvent.Kind} を破棄");
            return;
        }

        if (IsRunning) Fault($"プラント通知: {Describe(motionEvent.Kind)}");
        else Log(Protocol.LogSeverity.Warning, $"プラント通知: {Describe(motionEvent.Kind)}");
    }

    /// <summary>ウォッチドッグ。帰還が途絶えたことを検知する。制御周期より短い間隔で呼ぶこと。</summary>
    public void Tick(double nowMs)
    {
        if (!IsRunning) return;

        double watchdogMs = Math.Max(_options.WatchdogMs, 3.0 * _network.PeriodMs);
        double silenceMs = nowMs - _lastFeedbackLocalMs;
        if (silenceMs > watchdogMs)
        {
            Trace.Write(DiagnosticLevel.Error,
                $"ウォッチドッグ発火: 最後の帰還 (Seq {_estSeq}) から {silenceMs:0} ms > 許容 {watchdogMs:0} ms");
            Fault($"ウォッチドッグ: {watchdogMs:0} ms 帰還なし");
        }
        else if (!_silenceWarned && silenceMs > watchdogMs * 0.5)
        {
            // 発火する前に「途絶えつつある」ことを 1 度だけ出す。復帰したら OnFeedback 側で解除する。
            _silenceWarned = true;
            Trace.Write(DiagnosticLevel.Debug,
                $"帰還が途絶えている: 最後の帰還から {silenceMs:0} ms (ウォッチドッグ {watchdogMs:0} ms の半分を超過)");
        }
    }

    public ControllerStatus Snapshot(int downlinkQueueLength = 0) => new(
        Mode, Reason, CommandId, _volts, _targetX,
        _estX, CartPoleDynamics.WrapAngle(_estTheta),
        _feedbackInterval.Value, _uplinkDelay.Value, _e2eDelay.Value,
        _feedbackCount, _staleEventCount, downlinkQueueLength);

    // ---- 内部 ----

    private static string Describe(MotionEventKind kind) => kind switch
    {
        MotionEventKind.EmergencyStop => "非常停止",
        MotionEventKind.LimitReached => "メカリミット到達",
        MotionEventKind.CommandTimeout => "指令タイムアウト",
        MotionEventKind.DriveDisabled => "ドライブ無効中の指令拒否",
        _ => kind.ToString(),
    };

    /// <summary>設計結果のキャッシュ。UI のスライダで取りうる組合せは高々数十通りしかない。</summary>
    private static readonly ConcurrentDictionary<(PendulumParameters, int, double), ControlDesign> DesignCache = new();

    /// <summary>キャッシュの上限。想定外の入力で無限に増えないように抑える。</summary>
    private const int MaxCachedDesigns = 256;

    /// <summary>
    /// 帰還周期と振子長から離散 LQR ゲインと理論遅延余裕を設計する。
    /// コントローラ本体とテスト・解析ツールが同じ経路を通るように公開している。
    ///
    /// 1 回あたり数十 ms かかり、制御ループと同じスレッドで走る。
    /// スライダのドラッグで同じ組合せが何度も要求されるため、結果をキャッシュする
    /// (入力が同じなら出力も同じ純粋計算)。
    /// </summary>
    public static ControlDesign CreateDesign(PendulumParameters parameters, int periodMs, double velocityCutoffHz)
        => CreateDesign(parameters, periodMs, velocityCutoffHz, out _);

    /// <param name="cacheHit">キャッシュから返した場合 true。診断トレース用。</param>
    public static ControlDesign CreateDesign(
        PendulumParameters parameters, int periodMs, double velocityCutoffHz, out bool cacheHit)
    {
        var key = (parameters, periodMs, velocityCutoffHz);
        if (DesignCache.TryGetValue(key, out var cached))
        {
            cacheHit = true;
            return cached;
        }

        cacheHit = false;
        var model = LinearizedModel.Create(parameters);
        double ts = periodMs / 1000.0;
        var (ad, bd) = Discretization.ZeroOrderHold(model.A, model.B, ts);
        var gains = DiscreteLqr.Solve(ad, bd, StateWeights, InputWeight);
        int margin = DelayMarginAnalyzer.Compute(model.A, model.B, gains, ts, velocityCutoffHz);

        var design = new ControlDesign(ad, bd, new DesignInfo(gains, model.UnstablePole, margin, periodMs));
        if (DesignCache.Count < MaxCachedDesigns) DesignCache.TryAdd(key, design);
        return design;
    }

    private void Redesign(string cause)
    {
        long started = System.Diagnostics.Stopwatch.GetTimestamp();
        var design = CreateDesign(_parameters, _network.PeriodMs, _options.VelocityCutoffHz, out bool cacheHit);
        double elapsedMs = System.Diagnostics.Stopwatch.GetElapsedTime(started).TotalMilliseconds;

        _ad = design.Ad;
        _bd = design.Bd;
        _gains = design.Info.Gains;
        Design = design.Info;

        // 設計計算は制御ループと同じスレッドで走る。キャッシュミスで何十 ms 止まったかを残す。
        Trace.Write(cacheHit ? DiagnosticLevel.Information : DiagnosticLevel.Warning,
            $"再設計 ({cause}): 周期 {Design.PeriodMs} ms, 振子長 {_parameters.PendulumLength:0.00} m, LPF {_options.VelocityCutoffHz:0.#} Hz → " +
            $"K=[{_gains[0]:0.00}, {_gains[1]:0.00}, {_gains[2]:0.00}, {_gains[3]:0.00}], 不安定極 {Design.UnstablePole:0.00} rad/s, " +
            $"理論遅延余裕 {Design.DelayMarginMs} ms ({(cacheHit ? "キャッシュ" : "新規計算")}, {elapsedMs:0.0} ms{(cacheHit ? "" : " — 制御ループを止めた")})");

        DesignUpdated?.Invoke(Design);
    }

    private void Start(ControlMode target, double nowMs)
    {
        if (Mode != ControlMode.Idle)
        {
            Trace.Write(DiagnosticLevel.Warning, $"{target} の開始を拒否: {Mode} 中");
            Log(Protocol.LogSeverity.Warning,
                $"{Mode} 中は開始できません{(Mode == ControlMode.Fault ? " (異常リセットが必要)" : "")}");
            return;
        }

        CommandId++;
        _seq = 0;
        _voltageHistory.Clear();
        _startPlantMs = _lastPlantTimestampMs;
        _lastFeedbackLocalMs = nowMs;
        _feedbackInterval.Reset();
        _uplinkDelay.Reset();
        _e2eDelay.Reset();
        _estimatorReady = false;
        _estSeq = -1;
        _targetX = 0.0;
        _silenceWarned = false;
        _firstFeedbackTraced = false;
        _summaryDueAtPlantMs = double.NaN;

        Trace.Write(DiagnosticLevel.Information,
            $"運転開始 {target}: CommandId #{CommandId}, 開始時刻ガード {_startPlantMs:0} ms (プラント時計), " +
            $"周期 {_network.PeriodMs} ms, 予測器 {(_options.UsePredictor ? "ON" : "OFF")}, ウォッチドッグ {Math.Max(_options.WatchdogMs, 3.0 * _network.PeriodMs):0} ms, " +
            $"これまでの帰還 {_feedbackCount} 本");
        SetMode(target, $"CommandId #{CommandId}");
    }

    private void SetMode(ControlMode mode, string reason)
    {
        if (mode == Mode) return;
        var previous = Mode;
        Mode = mode;
        Reason = reason;

        Trace.Write(mode == ControlMode.Fault ? DiagnosticLevel.Error : DiagnosticLevel.Information,
            $"状態遷移 {previous} → {mode} ({reason}) CommandId #{CommandId}, 指令 {_seq} 本, 帰還 {_feedbackCount} 本, " +
            $"推定 x={_estX:+0.000;-0.000;+0.000} m θ={CartPoleDynamics.WrapAngle(_estTheta) * RadToDeg:+0.0;-0.0;+0.0}°");
        Log(mode == ControlMode.Fault ? Protocol.LogSeverity.Error : Protocol.LogSeverity.Info,
            $"{previous} → {mode}{(string.IsNullOrEmpty(reason) ? "" : $"  ({reason})")}");
    }

    private void Send(double volts, double basedOnMs)
    {
        double requested = volts;
        volts = CartPoleDynamics.Clamp(volts, -_parameters.MaxVoltage, _parameters.MaxVoltage);
        bool saturated = volts != requested;
        if (saturated) _saturatedCount++;

        _volts = volts;
        _voltageHistory.Add(volts);
        if (_voltageHistory.Count > HistoryCapacity) _voltageHistory.RemoveAt(0);
        _maxAbsVoltsSinceSummary = Math.Max(_maxAbsVoltsSinceSummary, Math.Abs(volts));

        if (Trace.IsEnabled(DiagnosticLevel.Trace))
        {
            Trace.Write(DiagnosticLevel.Trace,
                $"指令 #{CommandId}/{_seq} u={volts:+0.000;-0.000;+0.000} V basedOn={basedOnMs:0.0}" +
                $"{(saturated ? $" 飽和 (要求 {requested:+0.0;-0.0;+0.0} V)" : "")}" +
                $"{(_options.UsePredictor ? $" 予測 {_lastPredictSteps} step" : "")}");
        }

        VoltageProduced?.Invoke(new VoltageCommand(CommandId, _seq++, volts, basedOnMs));
    }

    private void Abort(string cause)
    {
        _volts = 0.0;
        Trace.Write(DiagnosticLevel.Information, $"AbortCommand #{CommandId} を送出 ({cause}): 電圧 0 へ");
        AbortProduced?.Invoke(new AbortCommand(CommandId));
    }

    private void Fault(string reason)
    {
        if (Mode == ControlMode.Fault)
        {
            Trace.Write(DiagnosticLevel.Debug, $"FAULT 中に再度の異常要因: {reason} (無視)");
            return;
        }
        Trace.Write(DiagnosticLevel.Error, $"FAULT: {reason}");
        Abort(reason);
        SetMode(ControlMode.Fault, reason);
    }

    private void Control(double measuredAtMs)
    {
        if (!IsRunning) return;
        double theta = CartPoleDynamics.WrapAngle(_estTheta);
        _maxAbsThetaSinceSummary = Math.Max(_maxAbsThetaSinceSummary, Math.Abs(theta));

        if (Mode == ControlMode.SwingUp)
        {
            if (Math.Abs(theta) < CaptureAngleRad && Math.Abs(_pendulumVelocity.Value) < CaptureRateRadPerSec)
            {
                Trace.Write(DiagnosticLevel.Information,
                    $"倒立領域に到達: |θ|={Math.Abs(theta) * RadToDeg:0.0}° < {CaptureAngleRad * RadToDeg:0}°, " +
                    $"|θ̇|={Math.Abs(_pendulumVelocity.Value):0.00} < {CaptureRateRadPerSec:0.0} rad/s, x={_estX:+0.000;-0.000;+0.000} m → LQR へ切替");
                SetMode(ControlMode.Balance, "倒立領域に到達");
            }
            else
            {
                if (Math.Abs(_estX) > _parameters.RailStroke - SoftLimitMarginM)
                {
                    Trace.Write(DiagnosticLevel.Error,
                        $"スイングアップ中にソフトリミット超過: |x|={Math.Abs(_estX):0.000} m > {_parameters.RailStroke - SoftLimitMarginM:0.000} m");
                    Fault("ソフトリミット超過");
                    return;
                }
                var state = new PlantState(_estX, _estTheta, _cartVelocity.Value, _pendulumVelocity.Value);
                double swingVolts = SwingUpController.Voltage(_parameters, _derived, state, _swingUpGains);

                if (Trace.IsEnabled(DiagnosticLevel.Trace))
                {
                    double energy = CartPoleDynamics.PendulumEnergy(_parameters, _derived, state);
                    Trace.Write(DiagnosticLevel.Trace,
                        $"スイングアップ: E={energy:+0.000;-0.000;+0.000} J (目標 {_swingUpGains.EnergyOffset:+0.000} J), θ={theta * RadToDeg:+0.0;-0.0;+0.0}°, θ̇={_pendulumVelocity.Value:+0.00;-0.00;+0.00} rad/s → u={swingVolts:+0.00;-0.00;+0.00} V");
                }
                Send(swingVolts, measuredAtMs);
                return;
            }
        }

        if (Math.Abs(theta) > FallAngleRad)
        {
            Trace.Write(DiagnosticLevel.Error,
                $"転倒検知: |θ|={Math.Abs(theta) * RadToDeg:0.0}° > {FallAngleRad * RadToDeg:0}°, θ̇={_pendulumVelocity.Value:+0.00;-0.00;+0.00} rad/s, x={_estX:+0.000;-0.000;+0.000} m, 直前の指令 {_volts:+0.0;-0.0;+0.0} V");
            Fault($"転倒検知 |θ|={Math.Abs(theta) * 180.0 / Math.PI:0}°");
            return;
        }
        if (Math.Abs(_estX) > _parameters.RailStroke - SoftLimitMarginM)
        {
            Trace.Write(DiagnosticLevel.Error,
                $"ソフトリミット超過: |x|={Math.Abs(_estX):0.000} m > {_parameters.RailStroke - SoftLimitMarginM:0.000} m, ẋ={_cartVelocity.Value:+0.00;-0.00;+0.00} m/s, 目標 {_targetX:+0.00;-0.00;+0.00} m");
            Fault("ソフトリミット超過");
            return;
        }

        double rampStep = TargetRampMetersPerSec * _network.PeriodMs / 1000.0;
        _targetX += CartPoleDynamics.Clamp(_options.CartTargetMeters - _targetX, -rampStep, rampStep);

        double[] x = [_estX - _targetX, theta, _cartVelocity.Value, _pendulumVelocity.Value];
        if (_options.UsePredictor)
        {
            double horizonMs = Math.Min(_e2eDelay.Value, MaxPredictionHorizonMs);
            int steps = (int)Math.Round(horizonMs / _network.PeriodMs);
            if (steps != _lastPredictSteps)
            {
                Trace.Write(DiagnosticLevel.Debug,
                    $"予測器の外挿量が {_lastPredictSteps}→{steps} step に変化 (E2E 平均 {_e2eDelay.Value:0.0} ms, 上限 {MaxPredictionHorizonMs:0} ms, 周期 {_network.PeriodMs} ms)");
                _lastPredictSteps = steps;
            }
            x = Predict(x, steps);
        }

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

    /// <summary>
    /// プラント時計で 1 秒ごとに、その間の動きを 1 行にまとめて出す。
    /// 帰還 1 本ごとの Trace を出さなくても「動いているか・どう動いているか」が分かるようにする。
    /// </summary>
    private void TraceSummaryIfDue(double plantMs, double dtSeconds)
    {
        _maxDtSinceSummary = Math.Max(_maxDtSinceSummary, dtSeconds * 1000.0);

        if (double.IsNaN(_summaryDueAtPlantMs))
        {
            _summaryDueAtPlantMs = plantMs + SummaryIntervalMs;
            ResetSummaryWindow();
            return;
        }
        if (plantMs < _summaryDueAtPlantMs) return;

        if (Trace.IsEnabled(DiagnosticLevel.Debug))
        {
            long feedbacks = _feedbackCount - _feedbackCountAtSummary;
            long commands = _seq - _seqAtSummary;
            long saturated = _saturatedCount - _saturatedAtSummary;
            Trace.Write(DiagnosticLevel.Debug,
                $"集計 {Mode} #{CommandId} t={plantMs / 1000.0:0.0}s: 帰還 {feedbacks} 本 (平均 {_feedbackInterval.Value:0.0} ms, 最大間隔 {_maxDtSinceSummary:0.0} ms), 指令 {commands} 本 (飽和 {saturated}), " +
                $"x={_estX:+0.000;-0.000;+0.000} m (目標 {_targetX:+0.00;-0.00;+0.00}), θ={CartPoleDynamics.WrapAngle(_estTheta) * RadToDeg:+0.0;-0.0;+0.0}° (最大 |θ| {_maxAbsThetaSinceSummary * RadToDeg:0.0}°), " +
                $"u={_volts:+0.00;-0.00;+0.00} V (最大 |u| {_maxAbsVoltsSinceSummary:0.0}), 上り {_uplinkDelay.Value:0.0} ms, E2E {_e2eDelay.Value:0.0} ms" +
                $"{(_options.UsePredictor ? $", 予測 {_lastPredictSteps} step" : "")}, 破棄 逆行 {_droppedBackwardCount}/開始前 {_droppedBeforeStartCount}");
        }

        ResetSummaryWindow();
        _summaryDueAtPlantMs = plantMs + SummaryIntervalMs;
    }

    private void ResetSummaryWindow()
    {
        _feedbackCountAtSummary = _feedbackCount;
        _seqAtSummary = _seq;
        _saturatedAtSummary = _saturatedCount;
        _maxAbsThetaSinceSummary = 0.0;
        _maxAbsVoltsSinceSummary = 0.0;
        _maxDtSinceSummary = 0.0;
    }

    private void Log(Protocol.LogSeverity level, string message)
        => Logged?.Invoke(new LogEntry(level, message, Mode, CommandId));
}

/// <summary>
/// 指数移動平均。初期化済みかどうかを「値が 0 か」で判定すると、
/// 0 が正当な観測値である量 (遅延 0 の構成での上り遅延など) で平滑が効かなくなるため、
/// 明示的なフラグを持つ。非有限のサンプルは推定値を汚さないよう捨てる。
/// </summary>
internal sealed class ExponentialAverage(double alpha)
{
    private bool _initialized;

    public double Value { get; private set; }

    public double Update(double sample)
    {
        if (!double.IsFinite(sample)) return Value;
        if (_initialized) Value += alpha * (sample - Value);
        else
        {
            Value = sample;
            _initialized = true;
        }
        return Value;
    }

    public void Reset()
    {
        _initialized = false;
        Value = 0.0;
    }
}
