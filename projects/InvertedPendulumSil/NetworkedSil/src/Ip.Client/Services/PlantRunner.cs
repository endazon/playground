using System.Threading.Channels;
using Ip.Shared;
using Ip.Shared.Diagnostics;
using Ip.Shared.Protocol;
using Ip.Shared.Simulation;
using Microsoft.AspNetCore.SignalR.Client;
using Microsoft.Extensions.Logging;

namespace Ip.Client.Services;

/// <summary>
/// ブラウザ側の仮想ハードウェア。プラントの固定ステップ積分と、コントローラとの通信を受け持つ。
///
/// ここには制御ロジックを一切置かない。電圧を決めるのはバックエンドだけ、というのが純 SIL 構成の要点であり、
/// 「描画が重いと制御が乱れる」といった実機では起きない結合を作らないためでもある。
///
/// 固定ステップループは <see cref="System.Threading.Timer"/> ではなく
/// <see cref="MonotonicClock"/> 基準の追いつき方式にしている。
/// WASM ではタイマの精度も最小間隔も保証されないため、「何 ms 経ったか」を毎回測って必要なステップ数だけ進める。
///
/// ログは 2 系統ある。画面のイベントログ (<see cref="Logs"/>) はオペレータ向けの少量のもの、
/// ブラウザの console (<c>ILogger</c>) は診断用で、プラントの積分・通信・時計同期の内部を出す
/// (カテゴリ <c>Ip.Client.Services.PlantRunner</c> / <c>Ip.Plant</c> / <c>Ip.Uplink</c>)。
/// </summary>
public sealed class PlantRunner : IAsyncDisposable
{
    /// <summary>ループの目標間隔 [ms]。実際にはブラウザのタイマ下限 (約 4ms) に丸められる。</summary>
    private const int LoopIntervalMs = 1;
    /// <summary>クロック同期の間隔 [ms]</summary>
    private const double ClockSyncIntervalMs = 5000.0;
    /// <summary>ループの生存ログ (周期・件数の集計) を出す間隔 [ms]</summary>
    private const double LoopSummaryIntervalMs = 1000.0;
    /// <summary>ループの 1 周回がこれより空いたら警告する [ms]。タブ非アクティブや GC の停止を可視化する。</summary>
    private const double LongGapWarnMs = 100.0;
    private const int MaxLogLines = 80;

    private readonly VirtualPlant _plant = new();
    private readonly ClockSynchronizer _clock = new();
    private readonly List<UiLogEntry> _logs = [];
    private readonly Channel<(string Method, object Payload)> _outbox =
        Channel.CreateUnbounded<(string, object)>(new UnboundedChannelOptions { SingleReader = true });
    private readonly DelayLine<object> _uplink;
    private readonly double _bootMs = MonotonicClock.NowMs;
    private readonly CancellationTokenSource _cts = new();
    private readonly ILogger<PlantRunner> _logger;

    private HubConnection? _hub;
    private Task? _loopTask;
    private Task? _sendTask;
    private double _nextClockSyncAt;

    // ---- ループの生存ログ用の集計 ----
    private long _loopCycles;
    private long _loopCyclesAtSummary;
    private double _lastLoopAt = double.NaN;
    private double _maxLoopGapMs;
    private double _lastLoopSummaryAt;
    private long _feedbackAtSummary;
    private long _sentFeedback, _sentMotion, _sentOther, _sendFailures, _sendDropped;
    private long _sentFeedbackAtSummary, _sendFailuresAtSummary;
    private long _receivedVoltage, _receivedAbort, _receivedDesign, _receivedStatus, _receivedLog;
    private long _receivedVoltageAtSummary, _receivedStatusAtSummary;
    private int _lastTimeSlips;
    private int _clockSyncFailures;

    public PlantRunner(ILoggerFactory loggerFactory)
    {
        _logger = loggerFactory.CreateLogger<PlantRunner>();

        // Ip.Shared の診断トレースをブラウザ console (ILogger) へ橋渡しする
        Attach(_plant.Trace, loggerFactory);
        var uplinkTrace = new DiagnosticTrace("uplink");
        Attach(uplinkTrace, loggerFactory);
        _uplink = new DelayLine<object>(Enqueue) { Trace = uplinkTrace };

        _plant.FeedbackReady += feedback => _uplink.Push(feedback, feedback.TimestampMs, Network);
        _plant.MotionEventRaised += motionEvent =>
        {
            _uplink.Push(motionEvent, motionEvent.TimestampMs, Network);
            AddLog("プラント", LogSeverity.Error, Describe(motionEvent.Kind));
        };

        _logger.LogInformation("PlantRunner 生成: 固定ステップ {Step} ms, ループ間隔 {Interval} ms, 時計同期 {ClockSync} s 間隔, プラント Trace={PlantTrace}, 上り Trace={UplinkTrace}",
            VirtualPlant.StepSeconds * 1000.0, LoopIntervalMs, ClockSyncIntervalMs / 1000.0, _plant.Trace.MinimumLevel, uplinkTrace.MinimumLevel);
    }

    /// <summary>画面操作 (プリセット適用など) を画面のイベントログに残す。</summary>
    public void NoteOperation(string message)
    {
        _logger.LogInformation("操作: {Message}", message);
        AddLog("操作", LogSeverity.Info, message);
    }

    /// <summary>UI に再描画のきっかけを渡す。頻度は購読側で間引く。</summary>
    public event Action? Changed;

    public NetworkConfig Network { get; private set; } = new();
    public ControlOptions Options { get; private set; } = new();
    public PlantConfig Plant { get; private set; } = new();

    /// <summary>バックエンドから届いた最新のコントローラ状態。未受信なら null。</summary>
    public ControllerStatus? Controller { get; private set; }

    /// <summary>バックエンドの設計結果 (LQR ゲイン・理論遅延余裕)。</summary>
    public DesignInfo? Design { get; private set; }

    public LinkState Link { get; private set; } = LinkState.Connecting;

    /// <summary>プラントの積分ループが異常終了したか。true なら表示は信用できない。</summary>
    public bool PlantStopped { get; private set; }
    public string LinkDetail { get; private set; } = "接続中";
    public double ClockOffsetMs => _clock.OffsetMs;
    public double ClockRoundTripMs => _clock.RoundTripMs;
    public IReadOnlyList<UiLogEntry> Logs => _logs;

    public PlantTelemetry Telemetry => new(
        _plant.SimSeconds, _plant.State.X, _plant.State.Theta, _plant.State.XDot, _plant.State.ThetaDot,
        _plant.AppliedVolts, _plant.DriveEnabled, _plant.EmergencyStopped, _plant.MeasuredE2EMs,
        _plant.RejectedCommands, _plant.AppliedCommands, _plant.TimeSlips, _uplink.Count);

    /// <summary>プラントの積分ループと通信を開始する。</summary>
    public async Task StartAsync(Uri hubUrl)
    {
        if (_hub is not null)
        {
            _logger.LogDebug("StartAsync: すでに開始済みなので何もしない ({State})", _hub.State);
            return;
        }

        _logger.LogInformation("Hub へ接続を開始: {Url} (MessagePack, 自動再接続, サーバタイムアウト 10 s / KeepAlive 3 s)", hubUrl);
        _hub = new HubConnectionBuilder()
            .WithUrl(hubUrl)
            .AddMessagePackProtocol()
            .WithAutomaticReconnect()
            // 既定の 30 秒では、通信が死んでも画面が「接続済み・倒立制御中」を表示し続ける。
            // サーバ側 (ClientTimeoutInterval 10s / KeepAlive 3s) と揃えて検知を早める。
            .WithServerTimeout(TimeSpan.FromSeconds(10))
            .WithKeepAliveInterval(TimeSpan.FromSeconds(3))
            .Build();

        _hub.On<VoltageCommand>(HubMethods.OnVoltage, command =>
        {
            _receivedVoltage++;
            if (_logger.IsEnabled(LogLevel.Trace))
                _logger.LogTrace("受信 OnVoltage #{CommandId}/{Seq} {Volts:+0.000;-0.000;+0.000} V basedOn={BasedOn:0.0} (受信時刻 {Now:0.0})",
                    command.CommandId, command.Seq, command.Volts, command.BasedOnMs, MonotonicClock.NowMs);
            _plant.ApplyCommand(command, MonotonicClock.NowMs);
        });
        _hub.On<AbortCommand>(HubMethods.OnAbort, abort =>
        {
            _receivedAbort++;
            _logger.LogInformation("受信 OnAbort #{CommandId}", abort.CommandId);
            _plant.ApplyAbort(abort);
        });
        _hub.On<DesignInfo>(HubMethods.OnDesign, design =>
        {
            _receivedDesign++;
            Design = design;
            string gains = string.Join(", ", design.Gains.Select(g => g.ToString("0.0")));
            _logger.LogInformation("受信 OnDesign: 周期 {Period} ms, K=[{Gains}], 不安定極 {Pole:0.00} rad/s, 理論遅延余裕 {Margin} ms",
                design.PeriodMs, gains, design.UnstablePole, design.DelayMarginMs);
            AddLog("バック", LogSeverity.Info,
                $"再設計: 周期 {design.PeriodMs} ms, K=[{gains}], 不安定極 {design.UnstablePole:0.00} rad/s, 遅延余裕 {design.DelayMarginMs} ms");
            Changed?.Invoke();
        });
        _hub.On<ControllerStatus>(HubMethods.OnStatus, status =>
        {
            _receivedStatus++;
            var previous = Controller;
            Controller = status;
            if (previous is null || previous.Mode != status.Mode || previous.CommandId != status.CommandId)
            {
                _logger.LogInformation("受信 OnStatus: {PrevMode} → {Mode} #{CommandId} ({Reason}), 帰還 {Feedback} 本, 上り {Uplink:0.0} ms, E2E {E2E:0.0} ms",
                    previous?.Mode.ToString() ?? "(初回)", status.Mode, status.CommandId, status.Reason,
                    status.FeedbackCount, status.UplinkDelayMs, status.E2EDelayMs);
            }
        });
        _hub.On<LogEntry>(HubMethods.OnLog, entry =>
        {
            _receivedLog++;
            _logger.LogInformation("受信 OnLog ({Level}, {Mode} #{CommandId}): {Message}", entry.Level, entry.Mode, entry.CommandId, entry.Message);
            AddLog("バック", entry.Level, entry.Message);
        });

        _hub.Reconnecting += ex =>
        {
            // 最後に受け取った状態を残すと、制御が死んでいるのに UI が「倒立制御中」と言い続ける。
            _logger.LogWarning(ex, "Hub 再接続中: 最後の状態 {Mode} を破棄する", Controller?.Mode.ToString() ?? "(なし)");
            Controller = null;
            SetLink(LinkState.Reconnecting, "再接続中");
            AddLog("システム", LogSeverity.Warning, "コントローラとの接続が切れました。再接続します。");
            return Task.CompletedTask;
        };
        _hub.Reconnected += async connectionId =>
        {
            _logger.LogInformation("Hub 再接続完了: 新しい接続 ID {ConnectionId}。設定を配り直し、時計同期をやり直す", connectionId ?? "(不明)");
            SetLink(LinkState.Connected, "接続済み");
            AddLog("システム", LogSeverity.Warning, "再接続しました。バック側は新しいセッションとして IDLE から始まります。");
            _clock.Reset();
            await PushConfigurationAsync().ConfigureAwait(false);
        };
        _hub.Closed += ex =>
        {
            if (ex is null) _logger.LogInformation("Hub 切断 (正常)");
            else _logger.LogError(ex, "Hub 切断 (異常): 自動再接続も諦めた");
            Controller = null;
            SetLink(LinkState.Disconnected, "切断");
            return Task.CompletedTask;
        };

        _sendTask = Task.Run(() => RunSendLoopAsync(_cts.Token));
        _loopTask = Task.Run(() => RunPlantLoopAsync(_cts.Token));

        try
        {
            double started = MonotonicClock.NowMs;
            await _hub.StartAsync(_cts.Token).ConfigureAwait(false);
            _logger.LogInformation("Hub 接続完了 ({Elapsed:0} ms): 接続 ID {ConnectionId}", MonotonicClock.NowMs - started, _hub.ConnectionId ?? "(不明)");
            SetLink(LinkState.Connected, "接続済み");
            await PushConfigurationAsync().ConfigureAwait(false);
            AddLog("システム", LogSeverity.Info, "コントローラに接続しました。");
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Hub 接続に失敗: {Url}", hubUrl);
            SetLink(LinkState.Disconnected, $"接続に失敗: {ex.Message}");
            AddLog("システム", LogSeverity.Error, $"接続に失敗しました: {ex.Message}");
        }
    }

    // ---- オペレータ操作 ----

    public Task SetNetworkAsync(NetworkConfig config)
    {
        var previous = Network;
        Network = config.Sanitized();
        if (Network != previous)
        {
            _logger.LogInformation("通信条件を変更: {Config} → プラントに適用し、バックへ送る", Network);
            if (Network.PeriodMs != previous.PeriodMs)
                AddLog("操作", LogSeverity.Info, $"帰還周期 {previous.PeriodMs} → {Network.PeriodMs} ms (バック側で再設計)");
        }
        _plant.Configure(Network);
        return SendAsync(HubMethods.ConfigureNetwork, Network);
    }

    public Task SetOptionsAsync(ControlOptions options)
    {
        var previous = Options;
        Options = options.Sanitized();
        if (Options != previous)
        {
            _logger.LogInformation("制御オプションを変更: {Options} → バックへ送る", Options);
            if (Options.UsePredictor != previous.UsePredictor)
                AddLog("操作", LogSeverity.Info, $"遅延補償 (予測器) を {(Options.UsePredictor ? "ON" : "OFF")}");
            if (Math.Abs(Options.CartTargetMeters - previous.CartTargetMeters) > 1e-9)
                AddLog("操作", LogSeverity.Info, $"台車の目標位置 {previous.CartTargetMeters:+0.00;-0.00;+0.00} → {Options.CartTargetMeters:+0.00;-0.00;+0.00} m");
        }
        return SendAsync(HubMethods.ConfigureControl, Options);
    }

    public Task SetPlantAsync(PlantConfig config)
    {
        var previous = Plant;
        Plant = config.Sanitized();
        if (Plant != previous)
        {
            _logger.LogInformation("プラント設定を変更: {Config} → プラントに適用し、バックの設計モデルにも送る", Plant);
            AddLog("操作", LogSeverity.Info, $"振子長 {previous.PendulumLength:0.00} → {Plant.PendulumLength:0.00} m (バック側で再設計)");
        }
        _plant.Configure(Plant);
        return SendAsync(HubMethods.ConfigurePlant, Plant);
    }

    public Task OperateAsync(OperatorAction action)
    {
        _logger.LogInformation("オペレータ操作 {Action} をバックへ送る (現在 {Mode}, 通信 {Link})", action, Controller?.Mode.ToString() ?? "(未受信)", Link);
        // 送信キューに積んでから画面ログを出す。AddLog は再描画を起こすので、操作より先に走らせない。
        var send = SendAsync(HubMethods.Operate, action);
        AddLog("操作", LogSeverity.Info, action switch
        {
            OperatorAction.SwingUp => "スイングアップ開始を指示",
            OperatorAction.Balance => "倒立制御開始を指示",
            OperatorAction.Stop => "停止を指示",
            OperatorAction.ClearFault => "異常リセットを指示",
            _ => $"{action} を指示",
        });
        return send;
    }

    /// <summary>「倒立から開始」: プラントを倒立姿勢に初期化してから開始指示を出す。</summary>
    public async Task StartFromUprightAsync()
    {
        _logger.LogInformation("倒立から開始: プラントを倒立姿勢にリセット → 30 ms 待つ → Balance を指示");
        _plant.Reset(upright: true);
        int epoch = _plant.Epoch;
        // リセット後の計測がバックに渡るまで待つ。この間に画面ログ (再描画) を挟むと WASM の単一スレッドを奪って
        // 帰還の送出が遅れ、リセット前の計測で運転を始めてしまうので、ログは開始指示の後に出す。
        await Task.Delay(30).ConfigureAwait(false);
        await OperateAsync(OperatorAction.Balance).ConfigureAwait(false);
        AddLog("プラント", LogSeverity.Info, $"倒立姿勢に初期化してから開始しました (Epoch {epoch})。");
    }

    /// <summary>「プラント初期化」: 現場側の復帰操作。停止させてからドライブを有効に戻す。</summary>
    public async Task InitializePlantAsync()
    {
        _logger.LogInformation("プラント初期化: Stop を指示 → 吊り下げ姿勢にリセットしてドライブを有効に戻す");
        await OperateAsync(OperatorAction.Stop).ConfigureAwait(false);
        _plant.Reset(upright: false);
        AddLog("プラント", LogSeverity.Info, $"プラントを初期化しました (ドライブ有効, Epoch {_plant.Epoch})。");
    }

    /// <summary>非常停止。通信を介さずプラント単独で電圧を切る。</summary>
    public void EmergencyStop()
    {
        _logger.LogWarning("非常停止ボタン: 通信を介さずプラント単独で電圧を切る");
        _plant.EmergencyStop(MonotonicClock.NowMs);
    }

    /// <summary>外乱。振子を指定方向に叩く。</summary>
    public void Push(int direction)
    {
        _logger.LogInformation("外乱: 振子を {Direction} へ押す", direction > 0 ? "+x 側" : "-x 側");
        AddLog("操作", LogSeverity.Info, $"外乱: 振子を{(direction > 0 ? "右 (+x)" : "左 (-x)")}へ押した");
        _plant.Push(direction);
    }

    // ---- 内部 ----

    private async Task RunPlantLoopAsync(CancellationToken token)
    {
        _logger.LogInformation("プラント積分ループ開始");
        _lastLoopSummaryAt = MonotonicClock.NowMs;
        try
        {
            while (!token.IsCancellationRequested)
            {
                double now = MonotonicClock.NowMs;
                _loopCycles++;
                if (!double.IsNaN(_lastLoopAt))
                {
                    double gap = now - _lastLoopAt;
                    _maxLoopGapMs = Math.Max(_maxLoopGapMs, gap);
                    if (gap > LongGapWarnMs)
                        _logger.LogWarning("積分ループが {Gap:0} ms 止まっていた (タブ非アクティブ / GC / 描画の詰まり)。プラントは追いつき方式で進める", gap);
                }
                _lastLoopAt = now;

                _plant.AdvanceTo(now);
                _uplink.Flush(now);

                if (_plant.TimeSlips != _lastTimeSlips)
                {
                    _lastTimeSlips = _plant.TimeSlips;
                    AddLog("プラント", LogSeverity.Warning, $"時間スリップ #{_plant.TimeSlips}: 実時間に追いつけず時間を捨てました (タブ非アクティブなど)。");
                }

                if (_hub?.State == HubConnectionState.Connected && now >= _nextClockSyncAt)
                {
                    _nextClockSyncAt = now + ClockSyncIntervalMs;
                    _ = SynchronizeClockAsync();
                }

                if (now - _lastLoopSummaryAt >= LoopSummaryIntervalMs) LogLoopSummary(now);

                await Task.Delay(LoopIntervalMs, token).ConfigureAwait(false);
            }
        }
        catch (OperationCanceledException)
        {
            _logger.LogInformation("プラント積分ループ停止 (破棄): {Cycles} 周回, {Steps} step, 帰還 {Feedback} 本", _loopCycles, _plant.StepCount, _plant.FeedbackCount);
        }
        catch (Exception ex)
        {
            // ここで黙って抜けると、描画ループは生きたまま台車と振子だけが凍る。
            // 画面上の手がかりが「シミュレーション時刻が進まない」ことだけになるので、必ず表に出す。
            PlantStopped = true;
            _logger.LogError(ex, "プラント積分ループが異常終了: sim t={Sim:0.000} s, {Cycles} 周回目", _plant.SimSeconds, _loopCycles);
            AddLog("システム", LogSeverity.Error, $"プラントの積分ループが停止しました: {ex.Message}");
        }
    }

    /// <summary>1 秒ごとに「積分ループが回っているか・通信が流れているか」を 1 行で残す。</summary>
    private void LogLoopSummary(double now)
    {
        if (_logger.IsEnabled(LogLevel.Debug))
        {
            long cycles = _loopCycles - _loopCyclesAtSummary;
            double elapsed = now - _lastLoopSummaryAt;
            _logger.LogDebug(
                "ループ集計 {Elapsed:0} ms: {Cycles} 周回 (平均 {Mean:0.0} ms, 最大間隔 {MaxGap:0.0} ms), sim t={Sim:0.0} s, " +
                "帰還 生成 {Generated} / 送信 {Sent} 本 (失敗 {Failures}), 上り遅延線 [{Uplink}], 受信 電圧 {Voltage} 本 / 状態 {Status} 回, " +
                "指令 適用 {Applied} / 拒否 {Rejected}, E2E {E2E:0.0} ms, 通信 {Link}, 時計 補正 {Offset:0.0} ms (RTT {Rtt:0.0} ms, {Samples} 標本)",
                elapsed, cycles, cycles > 0 ? elapsed / cycles : 0.0, _maxLoopGapMs, _plant.SimSeconds,
                _plant.FeedbackCount - _feedbackAtSummary, _sentFeedback - _sentFeedbackAtSummary, _sendFailures - _sendFailuresAtSummary,
                _uplink.DescribeStatistics(), _receivedVoltage - _receivedVoltageAtSummary, _receivedStatus - _receivedStatusAtSummary,
                _plant.AppliedCommands, _plant.RejectedCommands, _plant.MeasuredE2EMs, Link,
                _clock.OffsetMs, _clock.RoundTripMs, _clock.SampleCount);
        }

        _loopCyclesAtSummary = _loopCycles;
        _feedbackAtSummary = _plant.FeedbackCount;
        _sentFeedbackAtSummary = _sentFeedback;
        _sendFailuresAtSummary = _sendFailures;
        _receivedVoltageAtSummary = _receivedVoltage;
        _receivedStatusAtSummary = _receivedStatus;
        _maxLoopGapMs = 0.0;
        _lastLoopSummaryAt = now;
    }

    private async Task RunSendLoopAsync(CancellationToken token)
    {
        _logger.LogInformation("送信ループ開始");
        try
        {
            await foreach (var (method, payload) in _outbox.Reader.ReadAllAsync(token).ConfigureAwait(false))
            {
                if (_hub is null || _hub.State != HubConnectionState.Connected)
                {
                    // 帰還は捨てて構わないが、オペレータ操作が黙って消えるのは困る
                    _sendDropped++;
                    if (method == HubMethods.Operate)
                    {
                        _logger.LogWarning("未接続 ({State}) のため操作 {Action} を破棄", _hub?.State.ToString() ?? "未初期化", payload);
                        AddLog("システム", LogSeverity.Warning, "未接続のため操作を破棄しました。");
                    }
                    else if (_sendDropped == 1 || _sendDropped % 200 == 0)
                    {
                        _logger.LogDebug("未接続 ({State}) のため {Method} を破棄 (累計 {Dropped} 件)", _hub?.State.ToString() ?? "未初期化", method, _sendDropped);
                    }
                    continue;
                }

                switch (method)
                {
                    case HubMethods.SendFeedback:
                        _sentFeedback++;
                        if (_logger.IsEnabled(LogLevel.Trace) && payload is EncoderFeedback f)
                            _logger.LogTrace("送信 SendFeedback Seq {Seq} t={Timestamp:0.0} (送信処理時刻 {Now:0.0}, 遅延線で {Held:0.0} ms 保持)",
                                f.Seq, f.TimestampMs, MonotonicClock.NowMs, MonotonicClock.NowMs - f.TimestampMs);
                        break;
                    case HubMethods.SendMotionEvent:
                        _sentMotion++;
                        _logger.LogInformation("送信 SendMotionEvent: {Event}", payload);
                        break;
                    default:
                        _sentOther++;
                        _logger.LogDebug("送信 {Method}: {Payload}", method, payload);
                        break;
                }

                try
                {
                    await _hub.SendAsync(method, payload, token).ConfigureAwait(false);
                }
                catch (Exception ex) when (ex is not OperationCanceledException)
                {
                    // 切断中の取りこぼしは通信路の性質として許容する (再接続時に設定を配り直す)
                    _sendFailures++;
                    if (_sendFailures == 1 || _sendFailures % 100 == 0)
                        _logger.LogWarning(ex, "{Method} の送信に失敗 (累計 {Failures} 件)", method, _sendFailures);
                }
            }
        }
        catch (OperationCanceledException)
        {
            _logger.LogInformation("送信ループ停止: 帰還 {Feedback} / イベント {Motion} / その他 {Other} 本, 失敗 {Failures}, 未接続で破棄 {Dropped}",
                _sentFeedback, _sentMotion, _sentOther, _sendFailures, _sendDropped);
        }
    }

    private void Enqueue(object message)
    {
        string method = message switch
        {
            EncoderFeedback => HubMethods.SendFeedback,
            MotionEvent => HubMethods.SendMotionEvent,
            _ => string.Empty,
        };
        if (method.Length == 0)
        {
            _logger.LogWarning("上り遅延線から未知の型 {Type} が出てきたので捨てる", message.GetType().Name);
            return;
        }
        _outbox.Writer.TryWrite((method, message));
    }

    private Task SendAsync(string method, object payload)
    {
        _outbox.Writer.TryWrite((method, payload));
        return Task.CompletedTask;
    }

    private async Task PushConfigurationAsync()
    {
        _logger.LogDebug("設定をバックへ配る: {Network} / {Options} / {Plant}", Network, Options, Plant);
        await SendAsync(HubMethods.ConfigureNetwork, Network).ConfigureAwait(false);
        await SendAsync(HubMethods.ConfigureControl, Options).ConfigureAwait(false);
        await SendAsync(HubMethods.ConfigurePlant, Plant).ConfigureAwait(false);
        _nextClockSyncAt = 0.0;
    }

    /// <summary>
    /// 往復 4 時刻から時計のズレを推定し、サーバへ通知する。
    /// これを行わないと、サーバ側の「上り遅延」表示が 2 台の時計の差ぶんだけ嘘になる。
    /// </summary>
    private async Task SynchronizeClockAsync()
    {
        if (_hub is null) return;
        try
        {
            double sendAt = MonotonicClock.NowMs;
            var result = await _hub.InvokeAsync<ClockSyncResult>(HubMethods.SyncClock, sendAt, _cts.Token)
                .ConfigureAwait(false);
            double receivedAt = MonotonicClock.NowMs;
            bool accepted = _clock.Accept(result, receivedAt);

            _logger.LogDebug("時計同期 #{Sample}: RTT {Rtt:0.0} ms, オフセット {Offset:0.0} ms → {Verdict} (採用中: RTT {BestRtt:0.0} ms, 補正 {BestOffset:0.0} ms)",
                _clock.SampleCount + _clock.RejectedSampleCount, _clock.LastRoundTripMs, _clock.LastOffsetMs,
                accepted ? "採用" : _clock.LastRoundTripMs < 0.0 ? "RTT が負なので棄却" : "RTT が最良より大きいので不採用",
                _clock.RoundTripMs, _clock.OffsetMs);

            if (accepted)
            {
                if (_clock.SampleCount == 1)
                    AddLog("システム", LogSeverity.Info, $"時計同期: RTT {_clock.RoundTripMs:0.0} ms, サーバとの時計差 {_clock.OffsetMs:+0;-0;+0} ms");
                await SendAsync(HubMethods.ReportClockOffset, _clock.OffsetMs).ConfigureAwait(false);
            }
        }
        catch (Exception ex) when (ex is not OperationCanceledException)
        {
            // 同期に失敗しても制御そのものは続く (オフセットは前回値のまま)
            _clockSyncFailures++;
            _logger.LogWarning(ex, "時計同期に失敗 (累計 {Failures} 件)。オフセットは前回値 {Offset:0.0} ms のまま", _clockSyncFailures, _clock.OffsetMs);
        }
    }

    private void SetLink(LinkState state, string detail)
    {
        if (state != Link) _logger.LogInformation("通信状態 {Previous} → {State} ({Detail})", Link, state, detail);
        Link = state;
        LinkDetail = detail;
        Changed?.Invoke();
    }

    private void AddLog(string source, LogSeverity level, string message)
    {
        _logs.Insert(0, new UiLogEntry((MonotonicClock.NowMs - _bootMs) / 1000.0, source, level, message));
        if (_logs.Count > MaxLogLines) _logs.RemoveAt(_logs.Count - 1);
        Changed?.Invoke();
    }

    /// <summary>
    /// <c>Ip.Shared</c> の診断トレースを <see cref="ILogger"/> へ橋渡しする。
    /// カテゴリは <c>Ip.Plant</c> / <c>Ip.Uplink</c> になり、appsettings.json で個別に絞れる。
    /// </summary>
    private static void Attach(DiagnosticTrace trace, ILoggerFactory loggerFactory)
    {
        var logger = loggerFactory.CreateLogger("Ip." + char.ToUpperInvariant(trace.Source[0]) + trace.Source[1..]);
        trace.MinimumLevel = DiagnosticLevel.Trace;
        while (trace.MinimumLevel <= DiagnosticLevel.Error && !logger.IsEnabled(ToLogLevel(trace.MinimumLevel)))
            trace.MinimumLevel++;
        trace.Emitted += e => logger.Log(ToLogLevel(e.Level), "{Message}", e.Message);
    }

    private static LogLevel ToLogLevel(DiagnosticLevel level) => level switch
    {
        DiagnosticLevel.Trace => LogLevel.Trace,
        DiagnosticLevel.Debug => LogLevel.Debug,
        DiagnosticLevel.Information => LogLevel.Information,
        DiagnosticLevel.Warning => LogLevel.Warning,
        _ => LogLevel.Error,
    };

    private static string Describe(MotionEventKind kind) => kind switch
    {
        MotionEventKind.EmergencyStop => "非常停止",
        MotionEventKind.LimitReached => "メカリミット到達でドライブ遮断",
        MotionEventKind.CommandTimeout => "指令タイムアウトで電圧 0",
        MotionEventKind.DriveDisabled => "ドライブ無効中の指令を拒否",
        _ => kind.ToString(),
    };

    public async ValueTask DisposeAsync()
    {
        _logger.LogInformation("PlantRunner 破棄: sim t={Sim:0.000} s, {Steps} step, 帰還 {Feedback} 本, 指令 適用 {Applied} / 拒否 {Rejected}",
            _plant.SimSeconds, _plant.StepCount, _plant.FeedbackCount, _plant.AppliedCommands, _plant.RejectedCommands);
        await _cts.CancelAsync().ConfigureAwait(false);
        _outbox.Writer.TryComplete();
        if (_hub is not null) await _hub.DisposeAsync().ConfigureAwait(false);
        _cts.Dispose();
    }
}
