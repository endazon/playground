namespace Ip.Server.Sessions;

/// <summary>
/// 一定間隔でセッション一覧の要約をログに出す。
/// 「サーバは生きているか」「どの接続がどの状態で回っているか」を、UI を開かずにログだけで追えるようにする。
/// セッションが無いときは Debug、あるときは Information で出す。
/// </summary>
public sealed class SessionHeartbeat(
    ControllerSessionManager sessions,
    IConfiguration configuration,
    ILogger<SessionHeartbeat> logger) : BackgroundService
{
    /// <summary>既定の間隔 [s]。<c>Sil:HeartbeatSeconds</c> で変えられる。0 以下で無効。</summary>
    private const int DefaultIntervalSeconds = 10;

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        int seconds = configuration.GetValue("Sil:HeartbeatSeconds", DefaultIntervalSeconds);
        if (seconds <= 0)
        {
            logger.LogInformation("ハートビートは無効 (Sil:HeartbeatSeconds={Seconds})", seconds);
            return;
        }

        logger.LogInformation("ハートビート開始: {Seconds} 秒ごとにセッションの要約を出す", seconds);
        using var timer = new PeriodicTimer(TimeSpan.FromSeconds(seconds));
        try
        {
            while (await timer.WaitForNextTickAsync(stoppingToken).ConfigureAwait(false))
            {
                var list = sessions.Snapshot();
                if (list.Count == 0)
                {
                    logger.LogDebug("ハートビート: セッションなし (生成累計 {Created}, 拒否累計 {Rejected}, 異常累計 {Faulted})",
                        sessions.TotalCreated, sessions.TotalRejected, sessions.TotalFaulted);
                    continue;
                }

                logger.LogInformation("ハートビート: セッション {Count}/{Max} (生成累計 {Created}, 拒否累計 {Rejected}, 異常累計 {Faulted})",
                    list.Count, sessions.MaxSessions, sessions.TotalCreated, sessions.TotalRejected, sessions.TotalFaulted);
                foreach (var session in list)
                    logger.LogInformation("  {Summary}", session.Describe());
            }
        }
        catch (OperationCanceledException)
        {
            logger.LogInformation("ハートビート停止");
        }
    }
}
