using System.Collections.Concurrent;
using Microsoft.AspNetCore.SignalR;

namespace Ip.Server.Sessions;

/// <summary>接続 ID とコントローラを対応づける。接続が切れたらセッションごと破棄する。</summary>
public sealed class ControllerSessionManager(ILoggerFactory loggerFactory, IConfiguration configuration)
{
    /// <summary>同時に保持するセッション数の上限。1 接続ごとに制御ループが 1 本走るため上限を設ける。</summary>
    private readonly int _maxSessions = Math.Max(1, configuration.GetValue("Sil:MaxSessions", 64));

    private readonly ConcurrentDictionary<string, ControllerSession> _sessions = new();
    private readonly ILogger<ControllerSessionManager> _logger = loggerFactory.CreateLogger<ControllerSessionManager>();

    public int Count => _sessions.Count;

    /// <summary>上限に達している場合は null を返す。呼び出し側は接続を拒否すること。</summary>
    public ControllerSession? Create(string connectionId, IClientProxy client)
    {
        if (_sessions.Count >= _maxSessions)
        {
            _logger.LogWarning("セッション数の上限 {Max} に達したため接続を拒否しました {ConnectionId}",
                _maxSessions, connectionId);
            return null;
        }

        // ログのカテゴリがセッション自身になるよう、ファクトリから作る
        var session = new ControllerSession(connectionId, client, loggerFactory.CreateLogger<ControllerSession>());
        if (!_sessions.TryAdd(connectionId, session))
        {
            // 同じ接続 ID が二重に来ることは無いが、来たら新しい方を捨てて既存を使う
            _ = session.DisposeAsync().AsTask();
            return _sessions.TryGetValue(connectionId, out var existing) ? existing : null;
        }

        session.Faulted += id => _ = RemoveAsync(id).AsTask();
        _logger.LogInformation("セッション開始 {ConnectionId} (接続数 {Count})", connectionId, _sessions.Count);
        return session;
    }

    public ControllerSession? Get(string connectionId)
        => _sessions.TryGetValue(connectionId, out var session) ? session : null;

    public async ValueTask RemoveAsync(string connectionId)
    {
        if (!_sessions.TryRemove(connectionId, out var session)) return;
        await session.DisposeAsync().ConfigureAwait(false);
        _logger.LogInformation("セッション終了 {ConnectionId} (接続数 {Count})", connectionId, _sessions.Count);
    }
}
