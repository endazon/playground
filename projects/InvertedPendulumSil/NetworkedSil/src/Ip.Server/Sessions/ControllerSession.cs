using System.Diagnostics;
using System.Threading.Channels;
using Ip.Shared;
using Ip.Shared.Control;
using Ip.Shared.Diagnostics;
using Ip.Shared.Protocol;
using Ip.Shared.Simulation;
using Microsoft.AspNetCore.SignalR;

namespace Ip.Server.Sessions;

/// <summary>
/// 接続 1 本ぶんのコントローラ。SignalR の呼び出しは並行に来るため、
/// すべてを Channel に積んで単一のループで直列に処理する。
/// <see cref="ControllerCore"/> がスレッド安全でないのはこの前提があるからで、
/// ロックを撒くよりも「制御は 1 本のループで回す」という実機に近い構造を選んでいる。
///
/// ログは 3 つのカテゴリに分かれる:
/// <list type="bullet">
///   <item><c>Ip.Server.Sessions.ControllerSession</c> — ループの生存 (周期・停止時間・キュー・送受信の件数)</item>
///   <item><c>Ip.Controller</c> — コントローラ本体の診断トレース (状態遷移・帰還・指令・再設計)</item>
///   <item><c>Ip.Downlink</c> — 下り遅延線 (スパイク注入・配送)</item>
/// </list>
/// </summary>
public sealed class ControllerSession : IAsyncDisposable
{
    /// <summary>
    /// 制御ループの周回間隔 [ms]。ウォッチドッグと遅延線の解像度を決める。
    /// <c>Task.Delay(1)</c> の実測周期は OS のタイマ粒度に丸められて 4ms 前後になるため、
    /// 「1ms 周期で回る」とは仮定しないこと (帰還 1 本ごとの制御計算自体はイベント駆動で即時に行われる)。
    /// </summary>
    private const int LoopIntervalMs = 1;
    /// <summary>UI へ状態を配信する間隔 [ms]</summary>
    private const double StatusIntervalMs = 50.0;
    /// <summary>ループの生存ログ (周期・件数の集計) を出す間隔 [ms]</summary>
    private const double LoopSummaryIntervalMs = 1000.0;
    /// <summary>1 周回がこれより長くかかったら警告する [ms]。再設計や GC で制御が止まったことを可視化する。</summary>
    private const double SlowCycleWarnMs = 20.0;
    /// <summary>切断時にループの停止を待つ上限。Hub の OnDisconnectedAsync をここで長く止めない。</summary>
    private static readonly TimeSpan ShutdownTimeout = TimeSpan.FromSeconds(2);

    /// <summary>1 周で処理する受信メッセージの上限。これを超えたぶんは次の周回に回す。</summary>
    private const int MaxMessagesPerCycle = 64;
    /// <summary>受信キューの上限。溢れたら古いものから捨てる。</summary>
    private const int InboxCapacity = 2048;
    /// <summary>送信キューの上限。溢れたら古いものから捨てる (古い電圧指令に価値はない)。</summary>
    private const int OutboxCapacity = 512;

    private readonly Channel<object> _inbox = Channel.CreateBounded<object>(
        new BoundedChannelOptions(InboxCapacity) { SingleReader = true, FullMode = BoundedChannelFullMode.DropOldest });
    private readonly Channel<object> _outbox = Channel.CreateBounded<object>(
        new BoundedChannelOptions(OutboxCapacity) { SingleReader = true, FullMode = BoundedChannelFullMode.DropOldest });

    private readonly ControllerCore _core = new();
    private readonly DelayLine<object> _downlink;
    private readonly IClientProxy _client;
    private readonly ILogger _logger;
    private readonly CancellationTokenSource _cts = new();
    private readonly Task _controlLoop;
    private readonly Task _sendLoop;
    private readonly double _createdAtMs = MonotonicClock.NowMs;

    private NetworkConfig _network = new();
    /// <summary>最新の設計値の写し。Hub のスレッドから読むため、ControllerCore を直接触らない。</summary>
    private DesignInfo _design;
    /// <summary>最新の状態の写し。ハートビートなど別スレッドから読むため。</summary>
    private ControllerStatus? _latestStatus;
    private double _lastStatusAt;

    // ---- ループの生存ログ用の集計 ----
    private long _cycles;
    private long _cyclesAtSummary;
    private double _maxCycleGapMs;
    private double _maxCycleWorkMs;
    private double _lastSummaryAt;
    private double _lastCycleAt = double.NaN;
    private long _inboxDropped;
    private long _inboxDroppedAtSummary;
    private double _lastInboxDropWarnAt = double.NegativeInfinity;
    private int _maxInboxDepth;
    private long _handledFeedback, _handledMotion, _handledConfig, _handledOperate, _handledClock;
    private long _handledFeedbackAtSummary;
    private long _statusSent;
    private long _sentVoltage, _sentAbort, _sentDesign, _sentStatus, _sentLog, _sendFailures;
    private long _sentVoltageAtSummary, _sentStatusAtSummary, _sendFailuresAtSummary;
    private double _lastSendFailureWarnAt = double.NegativeInfinity;

    public ControllerSession(string connectionId, IClientProxy client, ILoggerFactory loggerFactory)
    {
        ConnectionId = connectionId;
        _client = client;
        _logger = loggerFactory.CreateLogger<ControllerSession>();

        // 診断トレースをカテゴリ別のロガーへ橋渡しする (Ip.Controller / Ip.Downlink)
        DiagnosticTraceBridge.Attach(_core.Trace, loggerFactory, connectionId);
        var downlinkTrace = new DiagnosticTrace("downlink");
        DiagnosticTraceBridge.Attach(downlinkTrace, loggerFactory, connectionId);
        _downlink = new DelayLine<object>(message => _outbox.Writer.TryWrite(message)) { Trace = downlinkTrace };

        // 制御に関わる下りだけ遅延線を通す。設計値・状態・ログは UI 表示用なので素通しでよい。
        _core.VoltageProduced += command => _downlink.Push(command, MonotonicClock.NowMs, _network);
        _core.AbortProduced += command => _downlink.Push(command, MonotonicClock.NowMs, _network);
        _core.DesignUpdated += design =>
        {
            Volatile.Write(ref _design, design);
            _outbox.Writer.TryWrite(design);
        };
        _design = _core.Design;
        _core.Logged += entry =>
        {
            _logger.LogDebug("[{ConnectionId}] UI ログ ({Level}, {Mode} #{CommandId}): {Message}",
                ConnectionId, entry.Level, entry.Mode, entry.CommandId, entry.Message);
            _outbox.Writer.TryWrite(entry);
        };

        _logger.LogInformation(
            "[{ConnectionId}] セッション生成: 受信キュー {Inbox} 件 / 送信キュー {Outbox} 件, ループ間隔 {Interval} ms, 状態配信 {Status} ms 間隔, " +
            "コントローラ Trace={CoreTrace}, 下り Trace={DownlinkTrace}",
            ConnectionId, InboxCapacity, OutboxCapacity, LoopIntervalMs, StatusIntervalMs,
            _core.Trace.MinimumLevel, downlinkTrace.MinimumLevel);

        _controlLoop = Task.Run(() => RunControlLoopAsync(_cts.Token));
        _sendLoop = Task.Run(() => RunSendLoopAsync(_cts.Token));
    }

    public string ConnectionId { get; }

    /// <summary>最新の状態の写し。未配信なら null。ハートビートの表示用。</summary>
    public ControllerStatus? LatestStatus => Volatile.Read(ref _latestStatus);

    /// <summary>生成からの経過秒。</summary>
    public double UptimeSeconds => (MonotonicClock.NowMs - _createdAtMs) / 1000.0;

    /// <summary>受信したメッセージを制御ループへ渡す。Hub のスレッドをブロックしない。</summary>
    public void Post(object message)
    {
        int depth = _inbox.Reader.Count;
        if (depth > _maxInboxDepth) _maxInboxDepth = depth;
        if (depth >= InboxCapacity)
        {
            // DropOldest なので TryWrite は成功するが、古いメッセージが黙って消える。それを可視化する。
            long dropped = Interlocked.Increment(ref _inboxDropped);
            double now = MonotonicClock.NowMs;
            if (now - _lastInboxDropWarnAt >= 1000.0)
            {
                _lastInboxDropWarnAt = now;
                _logger.LogWarning(
                    "[{ConnectionId}] 受信キューが満杯 ({Capacity} 件): 古いメッセージを破棄している (累計 {Dropped} 件)。制御ループが追いついていない",
                    ConnectionId, InboxCapacity, dropped);
            }
        }
        _inbox.Writer.TryWrite(message);
    }

    /// <summary>制御ループが異常終了したときに接続 ID を通知する。</summary>
    public event Action<string>? Faulted;

    /// <summary>接続直後に現在の設計値を配信する。</summary>
    public void PublishDesign()
    {
        var design = Volatile.Read(ref _design);
        _logger.LogDebug("[{ConnectionId}] 接続直後の設計値を配信: 周期 {Period} ms, 遅延余裕 {Margin} ms",
            ConnectionId, design.PeriodMs, design.DelayMarginMs);
        _outbox.Writer.TryWrite(design);
    }

    /// <summary>ハートビート用の 1 行要約。</summary>
    public string Describe()
    {
        var status = LatestStatus;
        string state = status is null
            ? "状態未配信"
            : $"{status.Mode} #{status.CommandId} 帰還 {status.FeedbackCount} 本 (周期 {status.FeedbackIntervalMs:0.0} ms, 上り {status.UplinkDelayMs:0.0} ms, E2E {status.E2EDelayMs:0.0} ms), " +
              $"u={status.Volts:+0.00;-0.00;+0.00} V, x={status.EstimatedX:+0.000;-0.000;+0.000} m, θ={status.EstimatedTheta * 180.0 / Math.PI:+0.0;-0.0;+0.0}°";
        return $"{ConnectionId}: 稼働 {UptimeSeconds:0} s, {state}, 周回 {Interlocked.Read(ref _cycles)}, 受信キュー {_inbox.Reader.Count} 件 (破棄 {Interlocked.Read(ref _inboxDropped)}), " +
               $"送信キュー {_outbox.Reader.Count} 件, 下り [{_downlink.DescribeStatistics()}]";
    }

    private async Task RunControlLoopAsync(CancellationToken token)
    {
        _logger.LogInformation("[{ConnectionId}] 制御ループ開始 (スレッド {Thread})", ConnectionId, Environment.CurrentManagedThreadId);
        _lastSummaryAt = MonotonicClock.NowMs;
        try
        {
            while (!token.IsCancellationRequested)
            {
                double now = MonotonicClock.NowMs;
                _cycles++;
                if (!double.IsNaN(_lastCycleAt)) _maxCycleGapMs = Math.Max(_maxCycleGapMs, now - _lastCycleAt);
                _lastCycleAt = now;

                // 受信が殺到しても Flush / Tick に必ず到達させる。
                int handled = 0;
                for (; handled < MaxMessagesPerCycle && _inbox.Reader.TryRead(out var message); handled++)
                    Handle(message, now);
                if (handled == MaxMessagesPerCycle)
                {
                    _logger.LogDebug("[{ConnectionId}] 1 周回の処理上限 {Max} 件に達した。残り {Remaining} 件は次の周回へ",
                        ConnectionId, MaxMessagesPerCycle, _inbox.Reader.Count);
                }

                _downlink.Flush(now);
                _core.Tick(now);

                if (now - _lastStatusAt >= StatusIntervalMs)
                {
                    _lastStatusAt = now;
                    var status = _core.Snapshot(_downlink.Count);
                    Volatile.Write(ref _latestStatus, status);
                    _outbox.Writer.TryWrite(status);
                    _statusSent++;
                }

                double workMs = MonotonicClock.NowMs - now;
                _maxCycleWorkMs = Math.Max(_maxCycleWorkMs, workMs);
                if (workMs > SlowCycleWarnMs)
                {
                    _logger.LogWarning(
                        "[{ConnectionId}] 制御ループの 1 周回に {Work:0.0} ms かかった (処理 {Handled} 件)。この間の帰還は次の周回まで待たされる",
                        ConnectionId, workMs, handled);
                }

                if (now - _lastSummaryAt >= LoopSummaryIntervalMs) LogLoopSummary(now);

                await Task.Delay(LoopIntervalMs, token).ConfigureAwait(false);
            }
        }
        catch (OperationCanceledException)
        {
            // 切断による正常終了
            _logger.LogInformation("[{ConnectionId}] 制御ループ停止 (切断): {Cycles} 周回, 帰還 {Feedback} 本処理, 状態配信 {Status} 回",
                ConnectionId, _cycles, _handledFeedback, _statusSent);
        }
        catch (Exception ex)
        {
            // ここで黙って終わると、セッションは生き続けるのに制御だけが死ぬ。
            // UI から見ると「最後の状態のまま正常に見える」ので、必ず外に出して破棄まで持っていく。
            _logger.LogError(ex, "[{ConnectionId}] 制御ループが異常終了しました ({Mode} #{CommandId}, {Cycles} 周回目)",
                ConnectionId, _core.Mode, _core.CommandId, _cycles);
            _outbox.Writer.TryWrite(new AbortCommand(_core.CommandId));
            _outbox.Writer.TryWrite(new LogEntry(LogSeverity.Error, "コントローラが異常終了しました", _core.Mode, _core.CommandId));
            _inbox.Writer.TryComplete();
            Faulted?.Invoke(ConnectionId);
        }
    }

    /// <summary>1 秒ごとに「ループが回っているか・どのくらい正確に回っているか」を 1 行で残す。</summary>
    private void LogLoopSummary(double now)
    {
        if (_logger.IsEnabled(LogLevel.Debug))
        {
            long cycles = _cycles - _cyclesAtSummary;
            double elapsed = now - _lastSummaryAt;
            _logger.LogDebug(
                "[{ConnectionId}] ループ集計 {Elapsed:0} ms: {Cycles} 周回 (平均 {Mean:0.0} ms, 最大間隔 {MaxGap:0.0} ms, 最大処理 {MaxWork:0.0} ms), " +
                "帰還 {Feedback} 本, 受信キュー 最大 {InboxMax} 件 (破棄 {InboxDropped}), 送信 電圧 {Voltage} 本 / 状態 {Status} 回 / 失敗 {Failures}, " +
                "下り遅延線 [{Downlink}], {Mode} #{CommandId}",
                ConnectionId, elapsed, cycles, cycles > 0 ? elapsed / cycles : 0.0, _maxCycleGapMs, _maxCycleWorkMs,
                _handledFeedback - _handledFeedbackAtSummary, _maxInboxDepth, _inboxDropped - _inboxDroppedAtSummary,
                Interlocked.Read(ref _sentVoltage) - _sentVoltageAtSummary, Interlocked.Read(ref _sentStatus) - _sentStatusAtSummary,
                Interlocked.Read(ref _sendFailures) - _sendFailuresAtSummary,
                _downlink.DescribeStatistics(), _core.Mode, _core.CommandId);
        }

        _cyclesAtSummary = _cycles;
        _handledFeedbackAtSummary = _handledFeedback;
        _inboxDroppedAtSummary = _inboxDropped;
        _sentVoltageAtSummary = Interlocked.Read(ref _sentVoltage);
        _sentStatusAtSummary = Interlocked.Read(ref _sentStatus);
        _sendFailuresAtSummary = Interlocked.Read(ref _sendFailures);
        _maxCycleGapMs = 0.0;
        _maxCycleWorkMs = 0.0;
        _maxInboxDepth = 0;
        _lastSummaryAt = now;
    }

    private void Handle(object message, double now)
    {
        switch (message)
        {
            case EncoderFeedback feedback:
                _handledFeedback++;
                _core.OnFeedback(feedback, now);
                break;
            case MotionEvent motionEvent:
                _handledMotion++;
                _logger.LogDebug("[{ConnectionId}] MotionEvent {Kind} を制御ループへ (#{CommandId})", ConnectionId, motionEvent.Kind, motionEvent.CommandId);
                _core.OnMotionEvent(motionEvent, now);
                break;
            case NetworkConfig network:
                _handledConfig++;
                _network = network.Sanitized();
                _logger.LogDebug("[{ConnectionId}] NetworkConfig を制御ループへ: {Config}", ConnectionId, _network);
                _core.SetNetwork(_network);
                break;
            case ControlOptions options:
                _handledConfig++;
                _logger.LogDebug("[{ConnectionId}] ControlOptions を制御ループへ: {Options}", ConnectionId, options);
                _core.SetOptions(options);
                break;
            case PlantConfig plant:
                _handledConfig++;
                _logger.LogDebug("[{ConnectionId}] PlantConfig を制御ループへ: {Plant}", ConnectionId, plant);
                _core.SetPlant(plant);
                break;
            case OperatorAction action:
                _handledOperate++;
                _logger.LogInformation("[{ConnectionId}] オペレータ操作 {Action} を制御ループへ (現在 {Mode})", ConnectionId, action, _core.Mode);
                _core.Operate(action, now);
                break;
            case ClockOffset offset:
                _handledClock++;
                _logger.LogDebug("[{ConnectionId}] クロックオフセット {Previous:0.0} → {Offset:0.0} ms", ConnectionId, _core.ClockOffsetMs, offset.Milliseconds);
                _core.ClockOffsetMs = offset.Milliseconds;
                break;
            default:
                _logger.LogWarning("[{ConnectionId}] 未知のメッセージ型 {Type} を無視", ConnectionId, message.GetType().Name);
                break;
        }
    }

    /// <summary>
    /// 送信は専用ループで直列化する。<c>SendAsync</c> を制御ループから撃ちっぱなしにすると
    /// 順序保証が崩れ、切断時の例外も拾えなくなる。
    /// </summary>
    private async Task RunSendLoopAsync(CancellationToken token)
    {
        _logger.LogInformation("[{ConnectionId}] 送信ループ開始", ConnectionId);
        try
        {
            await foreach (var message in _outbox.Reader.ReadAllAsync(token).ConfigureAwait(false))
            {
                var (method, payload) = message switch
                {
                    VoltageCommand command => (HubMethods.OnVoltage, (object)command),
                    AbortCommand abort => (HubMethods.OnAbort, abort),
                    DesignInfo design => (HubMethods.OnDesign, design),
                    ControllerStatus status => (HubMethods.OnStatus, status),
                    LogEntry entry => (HubMethods.OnLog, entry),
                    _ => (string.Empty, message),
                };
                if (method.Length == 0)
                {
                    _logger.LogWarning("[{ConnectionId}] 送信できない型 {Type} を送信キューから捨てた", ConnectionId, message.GetType().Name);
                    continue;
                }

                switch (message)
                {
                    case VoltageCommand c:
                        Interlocked.Increment(ref _sentVoltage);
                        if (_logger.IsEnabled(LogLevel.Trace))
                            _logger.LogTrace("[{ConnectionId}] 送信 OnVoltage #{CommandId}/{Seq} {Volts:+0.000;-0.000;+0.000} V (送信キュー残 {Pending})",
                                ConnectionId, c.CommandId, c.Seq, c.Volts, _outbox.Reader.Count);
                        break;
                    case AbortCommand a:
                        Interlocked.Increment(ref _sentAbort);
                        _logger.LogInformation("[{ConnectionId}] 送信 OnAbort #{CommandId}", ConnectionId, a.CommandId);
                        break;
                    case DesignInfo d:
                        Interlocked.Increment(ref _sentDesign);
                        _logger.LogInformation("[{ConnectionId}] 送信 OnDesign: 周期 {Period} ms, K=[{Gains}], 不安定極 {Pole:0.00} rad/s, 遅延余裕 {Margin} ms",
                            ConnectionId, d.PeriodMs, string.Join(", ", d.Gains.Select(g => g.ToString("0.00"))), d.UnstablePole, d.DelayMarginMs);
                        break;
                    case ControllerStatus:
                        Interlocked.Increment(ref _sentStatus);
                        break;
                    case LogEntry e:
                        Interlocked.Increment(ref _sentLog);
                        _logger.LogDebug("[{ConnectionId}] 送信 OnLog ({Level}): {Message}", ConnectionId, e.Level, e.Message);
                        break;
                }

                try
                {
                    await _client.SendAsync(method, payload, token).ConfigureAwait(false);
                }
                catch (Exception ex) when (ex is not OperationCanceledException)
                {
                    // 1 通の送信失敗でループを抜けると、以後この接続へは何も送られなくなるのに
                    // 制御ループだけが回り続け、UI からは正常に見えてしまう。
                    long failures = Interlocked.Increment(ref _sendFailures);
                    double now = MonotonicClock.NowMs;
                    if (now - _lastSendFailureWarnAt >= 1000.0)
                    {
                        _lastSendFailureWarnAt = now;
                        _logger.LogWarning(ex, "[{ConnectionId}] {Method} の送信に失敗 (累計 {Failures} 件)。切断中なら再接続で新しいセッションになる",
                            ConnectionId, method, failures);
                    }
                    else
                    {
                        _logger.LogDebug(ex, "[{ConnectionId}] {Method} の送信に失敗 (累計 {Failures} 件)", ConnectionId, method, failures);
                    }
                }
            }
            _logger.LogInformation("[{ConnectionId}] 送信ループ停止 (キュー完了)", ConnectionId);
        }
        catch (OperationCanceledException)
        {
            _logger.LogInformation(
                "[{ConnectionId}] 送信ループ停止 (切断): 電圧 {Voltage} / Abort {Abort} / 設計 {Design} / 状態 {Status} / ログ {Log} 本, 失敗 {Failures}",
                ConnectionId, _sentVoltage, _sentAbort, _sentDesign, _sentStatus, _sentLog, _sendFailures);
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "[{ConnectionId}] 送信ループを終了しました", ConnectionId);
        }
    }

    public async ValueTask DisposeAsync()
    {
        long started = Stopwatch.GetTimestamp();
        _logger.LogInformation("[{ConnectionId}] セッション破棄開始: 稼働 {Uptime:0.0} s, {Mode} #{CommandId}, 受信キュー残 {Inbox} 件, 送信キュー残 {Outbox} 件",
            ConnectionId, UptimeSeconds, _core.Mode, _core.CommandId, _inbox.Reader.Count, _outbox.Reader.Count);

        await _cts.CancelAsync().ConfigureAwait(false);
        _inbox.Writer.TryComplete();
        _outbox.Writer.TryComplete();

        try
        {
            await Task.WhenAll(_controlLoop, _sendLoop).WaitAsync(ShutdownTimeout).ConfigureAwait(false);
        }
        catch (TimeoutException)
        {
            // まだトークンを使っている可能性があるので CancellationTokenSource は破棄しない。
            // 停止しなかったこと自体が知りたい事実なので、握り潰さずに記録する。
            _logger.LogWarning("[{ConnectionId}] ループが {Timeout} で停止しませんでした (制御ループ {Control}, 送信ループ {Send})",
                ConnectionId, ShutdownTimeout, _controlLoop.Status, _sendLoop.Status);
            return;
        }
        catch (OperationCanceledException)
        {
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "[{ConnectionId}] セッションの停止中に例外が発生しました", ConnectionId);
        }

        _cts.Dispose();
        _logger.LogInformation("[{ConnectionId}] セッション破棄完了 ({Elapsed:0.0} ms): 周回 {Cycles}, 帰還 {Feedback} / イベント {Motion} / 設定 {Config} / 操作 {Operate} / 時計 {Clock} 件を処理, 受信キュー破棄 {Dropped} 件",
            ConnectionId, Stopwatch.GetElapsedTime(started).TotalMilliseconds,
            _cycles, _handledFeedback, _handledMotion, _handledConfig, _handledOperate, _handledClock, _inboxDropped);
    }

    /// <summary>クロック同期で求めたオフセットを制御ループへ渡すための内部メッセージ。</summary>
    public readonly record struct ClockOffset(double Milliseconds);
}
