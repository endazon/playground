using System.Threading.Channels;
using Ip.Shared;
using Ip.Shared.Protocol;
using Ip.Shared.Simulation;
using Microsoft.AspNetCore.SignalR.Client;

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
/// </summary>
public sealed class PlantRunner : IAsyncDisposable
{
    /// <summary>ループの目標間隔 [ms]。実際にはブラウザのタイマ下限 (約 4ms) に丸められる。</summary>
    private const int LoopIntervalMs = 1;
    /// <summary>クロック同期の間隔 [ms]</summary>
    private const double ClockSyncIntervalMs = 5000.0;
    private const int MaxLogLines = 80;

    private readonly VirtualPlant _plant = new();
    private readonly ClockSynchronizer _clock = new();
    private readonly List<UiLogEntry> _logs = [];
    private readonly Channel<(string Method, object Payload)> _outbox =
        Channel.CreateUnbounded<(string, object)>(new UnboundedChannelOptions { SingleReader = true });
    private readonly DelayLine<object> _uplink;
    private readonly double _bootMs = MonotonicClock.NowMs;
    private readonly CancellationTokenSource _cts = new();

    private HubConnection? _hub;
    private Task? _loopTask;
    private Task? _sendTask;
    private double _nextClockSyncAt;

    public PlantRunner()
    {
        _uplink = new DelayLine<object>(Enqueue);
        _plant.FeedbackReady += feedback => _uplink.Push(feedback, feedback.TimestampMs, Network);
        _plant.MotionEventRaised += motionEvent =>
        {
            _uplink.Push(motionEvent, motionEvent.TimestampMs, Network);
            AddLog("プラント", LogSeverity.Error, Describe(motionEvent.Kind));
        };
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
        if (_hub is not null) return;

        _hub = new HubConnectionBuilder()
            .WithUrl(hubUrl)
            .AddMessagePackProtocol()
            .WithAutomaticReconnect()
            .Build();

        _hub.On<VoltageCommand>(HubMethods.OnVoltage, command => _plant.ApplyCommand(command, MonotonicClock.NowMs));
        _hub.On<AbortCommand>(HubMethods.OnAbort, _plant.ApplyAbort);
        _hub.On<DesignInfo>(HubMethods.OnDesign, design => { Design = design; Changed?.Invoke(); });
        _hub.On<ControllerStatus>(HubMethods.OnStatus, status => { Controller = status; });
        _hub.On<LogEntry>(HubMethods.OnLog, entry => AddLog("バック", entry.Level, entry.Message));

        _hub.Reconnecting += _ =>
        {
            SetLink(LinkState.Reconnecting, "再接続中");
            AddLog("システム", LogSeverity.Warning, "コントローラとの接続が切れました。再接続します。");
            return Task.CompletedTask;
        };
        _hub.Reconnected += async _ =>
        {
            SetLink(LinkState.Connected, "接続済み");
            AddLog("システム", LogSeverity.Warning, "再接続しました。バック側は新しいセッションとして IDLE から始まります。");
            _clock.Reset();
            await PushConfigurationAsync().ConfigureAwait(false);
        };
        _hub.Closed += _ =>
        {
            SetLink(LinkState.Disconnected, "切断");
            return Task.CompletedTask;
        };

        _sendTask = Task.Run(() => RunSendLoopAsync(_cts.Token));
        _loopTask = Task.Run(() => RunPlantLoopAsync(_cts.Token));

        try
        {
            await _hub.StartAsync(_cts.Token).ConfigureAwait(false);
            SetLink(LinkState.Connected, "接続済み");
            await PushConfigurationAsync().ConfigureAwait(false);
            AddLog("システム", LogSeverity.Info, "コントローラに接続しました。");
        }
        catch (Exception ex)
        {
            SetLink(LinkState.Disconnected, $"接続に失敗: {ex.Message}");
            AddLog("システム", LogSeverity.Error, $"接続に失敗しました: {ex.Message}");
        }
    }

    // ---- オペレータ操作 ----

    public Task SetNetworkAsync(NetworkConfig config)
    {
        Network = config.Sanitized();
        _plant.Configure(Network);
        return SendAsync(HubMethods.ConfigureNetwork, Network);
    }

    public Task SetOptionsAsync(ControlOptions options)
    {
        Options = options.Sanitized();
        return SendAsync(HubMethods.ConfigureControl, Options);
    }

    public Task SetPlantAsync(PlantConfig config)
    {
        Plant = config.Sanitized();
        _plant.Configure(Plant);
        return SendAsync(HubMethods.ConfigurePlant, Plant);
    }

    public Task OperateAsync(OperatorAction action) => SendAsync(HubMethods.Operate, action);

    /// <summary>「倒立から開始」: プラントを倒立姿勢に初期化してから開始指示を出す。</summary>
    public async Task StartFromUprightAsync()
    {
        _plant.Reset(upright: true);
        await Task.Delay(30).ConfigureAwait(false); // リセット後の計測がバックに渡るまで待つ
        await OperateAsync(OperatorAction.Balance).ConfigureAwait(false);
    }

    /// <summary>「プラント初期化」: 現場側の復帰操作。停止させてからドライブを有効に戻す。</summary>
    public async Task InitializePlantAsync()
    {
        await OperateAsync(OperatorAction.Stop).ConfigureAwait(false);
        _plant.Reset(upright: false);
        AddLog("プラント", LogSeverity.Info, "プラントを初期化しました (ドライブ有効)。");
    }

    /// <summary>非常停止。通信を介さずプラント単独で電圧を切る。</summary>
    public void EmergencyStop() => _plant.EmergencyStop(MonotonicClock.NowMs);

    /// <summary>外乱。振子を指定方向に叩く。</summary>
    public void Push(int direction) => _plant.Push(direction);

    // ---- 内部 ----

    private async Task RunPlantLoopAsync(CancellationToken token)
    {
        try
        {
            while (!token.IsCancellationRequested)
            {
                double now = MonotonicClock.NowMs;
                _plant.AdvanceTo(now);
                _uplink.Flush(now);

                if (_hub?.State == HubConnectionState.Connected && now >= _nextClockSyncAt)
                {
                    _nextClockSyncAt = now + ClockSyncIntervalMs;
                    _ = SynchronizeClockAsync();
                }

                await Task.Delay(LoopIntervalMs, token).ConfigureAwait(false);
            }
        }
        catch (OperationCanceledException)
        {
        }
    }

    private async Task RunSendLoopAsync(CancellationToken token)
    {
        try
        {
            await foreach (var (method, payload) in _outbox.Reader.ReadAllAsync(token).ConfigureAwait(false))
            {
                if (_hub is null || _hub.State != HubConnectionState.Connected) continue;
                try
                {
                    await _hub.SendAsync(method, payload, token).ConfigureAwait(false);
                }
                catch (Exception ex) when (ex is not OperationCanceledException)
                {
                    // 切断中の取りこぼしは通信路の性質として許容する (再接続時に設定を配り直す)
                }
            }
        }
        catch (OperationCanceledException)
        {
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
        if (method.Length == 0) return;
        _outbox.Writer.TryWrite((method, message));
    }

    private Task SendAsync(string method, object payload)
    {
        _outbox.Writer.TryWrite((method, payload));
        return Task.CompletedTask;
    }

    private async Task PushConfigurationAsync()
    {
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
            if (_clock.Accept(result, MonotonicClock.NowMs))
                await SendAsync(HubMethods.ReportClockOffset, _clock.OffsetMs).ConfigureAwait(false);
        }
        catch (Exception ex) when (ex is not OperationCanceledException)
        {
            // 同期に失敗しても制御そのものは続く (オフセットは前回値のまま)
        }
    }

    private void SetLink(LinkState state, string detail)
    {
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
        await _cts.CancelAsync().ConfigureAwait(false);
        _outbox.Writer.TryComplete();
        if (_hub is not null) await _hub.DisposeAsync().ConfigureAwait(false);
        _cts.Dispose();
    }
}
