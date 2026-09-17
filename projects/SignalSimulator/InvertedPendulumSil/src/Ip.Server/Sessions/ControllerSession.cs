using System.Threading.Channels;
using Ip.Shared;
using Ip.Shared.Control;
using Ip.Shared.Protocol;
using Ip.Shared.Simulation;
using Microsoft.AspNetCore.SignalR;

namespace Ip.Server.Sessions;

/// <summary>
/// 接続 1 本ぶんのコントローラ。SignalR の呼び出しは並行に来るため、
/// すべてを Channel に積んで単一のループで直列に処理する。
/// <see cref="ControllerCore"/> がスレッド安全でないのはこの前提があるからで、
/// ロックを撒くよりも「制御は 1 本のループで回す」という実機に近い構造を選んでいる。
/// </summary>
public sealed class ControllerSession : IAsyncDisposable
{
    /// <summary>制御ループの周回間隔 [ms]。帰還処理はイベント駆動なので、ここはウォッチドッグと遅延線の解像度を決める。</summary>
    private const int LoopIntervalMs = 1;
    /// <summary>UI へ状態を配信する間隔 [ms]</summary>
    private const double StatusIntervalMs = 50.0;

    private readonly Channel<object> _inbox = Channel.CreateUnbounded<object>(
        new UnboundedChannelOptions { SingleReader = true });
    private readonly Channel<object> _outbox = Channel.CreateUnbounded<object>(
        new UnboundedChannelOptions { SingleReader = true });

    private readonly ControllerCore _core = new();
    private readonly DelayLine<object> _downlink;
    private readonly IClientProxy _client;
    private readonly ILogger _logger;
    private readonly CancellationTokenSource _cts = new();
    private readonly Task _controlLoop;
    private readonly Task _sendLoop;

    private NetworkConfig _network = new();
    private double _lastStatusAt;

    public ControllerSession(string connectionId, IClientProxy client, ILogger logger)
    {
        ConnectionId = connectionId;
        _client = client;
        _logger = logger;

        _downlink = new DelayLine<object>(message => _outbox.Writer.TryWrite(message));

        // 制御に関わる下りだけ遅延線を通す。設計値・状態・ログは UI 表示用なので素通しでよい。
        _core.VoltageProduced += command => _downlink.Push(command, MonotonicClock.NowMs, _network);
        _core.AbortProduced += command => _downlink.Push(command, MonotonicClock.NowMs, _network);
        _core.DesignUpdated += design => _outbox.Writer.TryWrite(design);
        _core.Logged += entry => _outbox.Writer.TryWrite(entry);

        _controlLoop = Task.Run(() => RunControlLoopAsync(_cts.Token));
        _sendLoop = Task.Run(() => RunSendLoopAsync(_cts.Token));
    }

    public string ConnectionId { get; }

    /// <summary>受信したメッセージを制御ループへ渡す。Hub のスレッドをブロックしない。</summary>
    public void Post(object message) => _inbox.Writer.TryWrite(message);

    /// <summary>接続直後に現在の設計値を配信する。</summary>
    public void PublishDesign() => _outbox.Writer.TryWrite(_core.Design);

    private async Task RunControlLoopAsync(CancellationToken token)
    {
        try
        {
            while (!token.IsCancellationRequested)
            {
                double now = MonotonicClock.NowMs;

                while (_inbox.Reader.TryRead(out var message)) Handle(message, now);

                _downlink.Flush(now);
                _core.Tick(now);

                if (now - _lastStatusAt >= StatusIntervalMs)
                {
                    _lastStatusAt = now;
                    _outbox.Writer.TryWrite(_core.Snapshot(_downlink.Count));
                }

                await Task.Delay(LoopIntervalMs, token).ConfigureAwait(false);
            }
        }
        catch (OperationCanceledException)
        {
            // 切断による正常終了
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "制御ループが異常終了しました ({ConnectionId})", ConnectionId);
        }
    }

    private void Handle(object message, double now)
    {
        switch (message)
        {
            case EncoderFeedback feedback:
                _core.OnFeedback(feedback, now);
                break;
            case MotionEvent motionEvent:
                _core.OnMotionEvent(motionEvent, now);
                break;
            case NetworkConfig network:
                _network = network.Sanitized();
                _core.SetNetwork(_network);
                break;
            case ControlOptions options:
                _core.SetOptions(options);
                break;
            case PlantConfig plant:
                _core.SetPlant(plant);
                break;
            case OperatorAction action:
                _core.Operate(action, now);
                break;
            case ClockOffset offset:
                _core.ClockOffsetMs = offset.Milliseconds;
                break;
        }
    }

    /// <summary>
    /// 送信は専用ループで直列化する。<c>SendAsync</c> を制御ループから撃ちっぱなしにすると
    /// 順序保証が崩れ、切断時の例外も拾えなくなる。
    /// </summary>
    private async Task RunSendLoopAsync(CancellationToken token)
    {
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
                if (method.Length == 0) continue;

                await _client.SendAsync(method, payload, token).ConfigureAwait(false);
            }
        }
        catch (OperationCanceledException)
        {
        }
        catch (Exception ex)
        {
            _logger.LogDebug(ex, "送信ループを終了しました ({ConnectionId})", ConnectionId);
        }
    }

    public async ValueTask DisposeAsync()
    {
        await _cts.CancelAsync().ConfigureAwait(false);
        _inbox.Writer.TryComplete();
        _outbox.Writer.TryComplete();
        await Task.WhenAll(_controlLoop, _sendLoop).WaitAsync(TimeSpan.FromSeconds(2))
            .ContinueWith(_ => { }, TaskScheduler.Default).ConfigureAwait(false);
        _cts.Dispose();
    }

    /// <summary>クロック同期で求めたオフセットを制御ループへ渡すための内部メッセージ。</summary>
    public readonly record struct ClockOffset(double Milliseconds);
}
