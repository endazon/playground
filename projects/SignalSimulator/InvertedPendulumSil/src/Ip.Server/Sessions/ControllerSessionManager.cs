using System.Collections.Concurrent;
using Microsoft.AspNetCore.SignalR;

namespace Ip.Server.Sessions;

/// <summary>接続 ID とコントローラを対応づける。接続が切れたらセッションごと破棄する。</summary>
public sealed class ControllerSessionManager(ILogger<ControllerSessionManager> logger)
{
    private readonly ConcurrentDictionary<string, ControllerSession> _sessions = new();

    public int Count => _sessions.Count;

    public ControllerSession Create(string connectionId, IClientProxy client)
    {
        var session = new ControllerSession(connectionId, client, logger);
        if (!_sessions.TryAdd(connectionId, session))
        {
            // 同じ接続 ID が二重に来ることは無いが、来たら新しい方を捨てて既存を返す
            _ = session.DisposeAsync().AsTask();
            return _sessions[connectionId];
        }
        logger.LogInformation("セッション開始 {ConnectionId} (接続数 {Count})", connectionId, _sessions.Count);
        return session;
    }

    public ControllerSession? Get(string connectionId)
        => _sessions.TryGetValue(connectionId, out var session) ? session : null;

    public async ValueTask RemoveAsync(string connectionId)
    {
        if (!_sessions.TryRemove(connectionId, out var session)) return;
        await session.DisposeAsync().ConfigureAwait(false);
        logger.LogInformation("セッション終了 {ConnectionId} (接続数 {Count})", connectionId, _sessions.Count);
    }
}
