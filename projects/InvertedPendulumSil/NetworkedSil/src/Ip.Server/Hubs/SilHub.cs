using Ip.Server.Sessions;
using Ip.Shared;
using Ip.Shared.Protocol;
using Microsoft.AspNetCore.Http.Connections.Features;
using Microsoft.AspNetCore.SignalR;

namespace Ip.Server.Hubs;

/// <summary>
/// プラント (Blazor WASM) とコントローラ (このサーバ) をつなぐ通信路。
///
/// Hub のメソッドは受け取ったものをセッションの Channel に積むだけで、制御計算は一切しない。
/// SignalR の呼び出しスレッドで重い処理をすると、他の接続の受信まで遅れるため。
///
/// ログは「何が届いたか」を受信の入口で残す。定周期の帰還は Trace (数百行/秒)、
/// 設定・操作・時計同期は Debug、接続の開始と終了は Information。
/// </summary>
public sealed class SilHub(ControllerSessionManager sessions, ILogger<SilHub> logger) : Hub
{
    public override Task OnConnectedAsync()
    {
        var http = Context.GetHttpContext();
        var transport = Context.Features.Get<IHttpTransportFeature>()?.TransportType;
        logger.LogInformation(
            "接続 {ConnectionId}: {Transport} from {RemoteIp}, UA={UserAgent}",
            Context.ConnectionId, transport?.ToString() ?? "不明", http?.Connection.RemoteIpAddress?.ToString() ?? "不明",
            Truncate(http?.Request.Headers.UserAgent.ToString(), 80));

        var session = sessions.Create(Context.ConnectionId, Clients.Caller);
        if (session is null)
        {
            // セッション数の上限。受け入れられない接続は即座に切る。
            logger.LogWarning("接続 {ConnectionId} を切断: セッションを生成できない (上限 {Max})", Context.ConnectionId, sessions.MaxSessions);
            Context.Abort();
            return Task.CompletedTask;
        }

        session.PublishDesign();
        return base.OnConnectedAsync();
    }

    public override async Task OnDisconnectedAsync(Exception? exception)
    {
        if (exception is not null)
            logger.LogWarning(exception, "接続が異常終了しました {ConnectionId}", Context.ConnectionId);
        else
            logger.LogInformation("切断 {ConnectionId} (正常)", Context.ConnectionId);

        await sessions.RemoveAsync(Context.ConnectionId).ConfigureAwait(false);
        await base.OnDisconnectedAsync(exception).ConfigureAwait(false);
    }

    /// <summary>定周期のエンコーダ帰還。</summary>
    public void SendFeedback(EncoderFeedback feedback)
    {
        if (logger.IsEnabled(LogLevel.Trace))
        {
            logger.LogTrace("受信 SendFeedback {ConnectionId}: Seq {Seq} #{CommandId} t={Timestamp:0.0} cart={Cart} pend={Pend} e2e={E2E:0.0} drive={Drive} epoch={Epoch} (受信時刻 {Now:0.0})",
                Context.ConnectionId, feedback.Seq, feedback.CommandId, feedback.TimestampMs, feedback.CartCounts, feedback.PendulumCounts,
                feedback.E2EMs, feedback.DriveEnabled, feedback.Epoch, MonotonicClock.NowMs);
        }
        Post(feedback);
    }

    /// <summary>プラント側の保護動作の通知。</summary>
    public void SendMotionEvent(MotionEvent motionEvent)
    {
        logger.LogInformation("受信 SendMotionEvent {ConnectionId}: {Kind} #{CommandId} t={Timestamp:0.0}",
            Context.ConnectionId, motionEvent.Kind, motionEvent.CommandId, motionEvent.TimestampMs);
        Post(motionEvent);
    }

    /// <summary>帰還周期・通信路条件の変更。周期が変わるとバック側で離散 LQR を再設計する。</summary>
    public void ConfigureNetwork(NetworkConfig config)
    {
        var sanitized = config.Sanitized();
        logger.LogDebug("受信 ConfigureNetwork {ConnectionId}: {Config}{Note}",
            Context.ConnectionId, sanitized, sanitized == config ? "" : $" (矯正前: {config})");
        Post(sanitized);
    }

    /// <summary>予測器の有無・台車目標位置などの変更。</summary>
    public void ConfigureControl(ControlOptions options)
    {
        var sanitized = options.Sanitized();
        logger.LogDebug("受信 ConfigureControl {ConnectionId}: {Options}{Note}",
            Context.ConnectionId, sanitized, sanitized == options ? "" : $" (矯正前: {options})");
        Post(sanitized);
    }

    /// <summary>振子長など、設計モデルに効くプラントパラメータの変更。</summary>
    public void ConfigurePlant(PlantConfig config)
    {
        var sanitized = config.Sanitized();
        logger.LogDebug("受信 ConfigurePlant {ConnectionId}: {Config}{Note}",
            Context.ConnectionId, sanitized, sanitized == config ? "" : $" (矯正前: {config})");
        Post(sanitized);
    }

    /// <summary>オペレータ操作 (開始・停止・異常リセット)。</summary>
    public void Operate(OperatorAction action)
    {
        logger.LogInformation("受信 Operate {ConnectionId}: {Action}", Context.ConnectionId, action);
        Post(action);
    }

    /// <summary>
    /// クロック同期。受信時刻と送信時刻を返すだけで、オフセットの計算はクライアント側で行う。
    /// サーバは複数クライアントを相手にするので、片側で完結させた方が状態を持たずに済む。
    /// </summary>
    public ClockSyncResult SyncClock(double clientSendMs)
    {
        double receivedAt = MonotonicClock.NowMs;
        var result = new ClockSyncResult(clientSendMs, receivedAt, MonotonicClock.NowMs);
        logger.LogDebug("受信 SyncClock {ConnectionId}: client={ClientSend:0.0} → server recv={Recv:0.0} send={Send:0.0} (サーバ内処理 {Hold:0.000} ms)",
            Context.ConnectionId, clientSendMs, result.ServerReceiveMs, result.ServerSendMs, result.ServerSendMs - result.ServerReceiveMs);
        return result;
    }

    /// <summary>
    /// クライアントが推定した「プラント時刻 → サーバ時刻」のオフセットを受け取る。
    /// この値は表示にしか使わないが、NaN が入ると表示が恒久的に壊れるので受信側で矯正する。
    /// </summary>
    public void ReportClockOffset(double offsetMs)
    {
        double sanitized = Sanitize.Finite(offsetMs, -MaxClockOffsetMs, MaxClockOffsetMs, 0.0);
        logger.LogDebug("受信 ReportClockOffset {ConnectionId}: {Offset:0.0} ms{Note}",
            Context.ConnectionId, sanitized, sanitized == offsetMs ? "" : $" (矯正前: {offsetMs})");
        Post(new ControllerSession.ClockOffset(sanitized));
    }

    /// <summary>許容するクロックオフセットの絶対値 [ms] (24 時間)。</summary>
    private const double MaxClockOffsetMs = 24.0 * 60.0 * 60.0 * 1000.0;

    private void Post(object message)
    {
        var session = sessions.Get(Context.ConnectionId);
        if (session is null)
        {
            logger.LogDebug("セッションが無い接続からの受信 {Type} を破棄しました {ConnectionId}", message.GetType().Name, Context.ConnectionId);
            return;
        }
        session.Post(message);
    }

    private static string Truncate(string? value, int max)
        => string.IsNullOrEmpty(value) ? "不明" : value.Length <= max ? value : value[..max] + "…";
}
