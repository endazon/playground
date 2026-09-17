using Ip.Server.Sessions;
using Ip.Shared;
using Ip.Shared.Protocol;
using Microsoft.AspNetCore.SignalR;

namespace Ip.Server.Hubs;

/// <summary>
/// プラント (Blazor WASM) とコントローラ (このサーバ) をつなぐ通信路。
///
/// Hub のメソッドは受け取ったものをセッションの Channel に積むだけで、制御計算は一切しない。
/// SignalR の呼び出しスレッドで重い処理をすると、他の接続の受信まで遅れるため。
/// </summary>
public sealed class SilHub(ControllerSessionManager sessions, ILogger<SilHub> logger) : Hub
{
    public override Task OnConnectedAsync()
    {
        var session = sessions.Create(Context.ConnectionId, Clients.Caller);
        if (session is null)
        {
            // セッション数の上限。受け入れられない接続は即座に切る。
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

        await sessions.RemoveAsync(Context.ConnectionId).ConfigureAwait(false);
        await base.OnDisconnectedAsync(exception).ConfigureAwait(false);
    }

    /// <summary>定周期のエンコーダ帰還。</summary>
    public void SendFeedback(EncoderFeedback feedback) => Post(feedback);

    /// <summary>プラント側の保護動作の通知。</summary>
    public void SendMotionEvent(MotionEvent motionEvent) => Post(motionEvent);

    /// <summary>帰還周期・通信路条件の変更。周期が変わるとバック側で離散 LQR を再設計する。</summary>
    public void ConfigureNetwork(NetworkConfig config) => Post(config.Sanitized());

    /// <summary>予測器の有無・台車目標位置などの変更。</summary>
    public void ConfigureControl(ControlOptions options) => Post(options.Sanitized());

    /// <summary>振子長など、設計モデルに効くプラントパラメータの変更。</summary>
    public void ConfigurePlant(PlantConfig config) => Post(config.Sanitized());

    /// <summary>オペレータ操作 (開始・停止・異常リセット)。</summary>
    public void Operate(OperatorAction action) => Post(action);

    /// <summary>
    /// クロック同期。受信時刻と送信時刻を返すだけで、オフセットの計算はクライアント側で行う。
    /// サーバは複数クライアントを相手にするので、片側で完結させた方が状態を持たずに済む。
    /// </summary>
    public ClockSyncResult SyncClock(double clientSendMs)
    {
        double receivedAt = MonotonicClock.NowMs;
        return new ClockSyncResult(clientSendMs, receivedAt, MonotonicClock.NowMs);
    }

    /// <summary>
    /// クライアントが推定した「プラント時刻 → サーバ時刻」のオフセットを受け取る。
    /// この値は表示にしか使わないが、NaN が入ると表示が恒久的に壊れるので受信側で矯正する。
    /// </summary>
    public void ReportClockOffset(double offsetMs)
        => Post(new ControllerSession.ClockOffset(
            Sanitize.Finite(offsetMs, -MaxClockOffsetMs, MaxClockOffsetMs, 0.0)));

    /// <summary>許容するクロックオフセットの絶対値 [ms] (24 時間)。</summary>
    private const double MaxClockOffsetMs = 24.0 * 60.0 * 60.0 * 1000.0;

    private void Post(object message)
    {
        var session = sessions.Get(Context.ConnectionId);
        if (session is null)
        {
            logger.LogDebug("セッションが無い接続からの受信を破棄しました {ConnectionId}", Context.ConnectionId);
            return;
        }
        session.Post(message);
    }
}
