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
    private long _totalCreated;
    private long _totalRejected;
    private long _totalFaulted;

    public int Count => _sessions.Count;

    /// <summary>設定された上限。起動ログとハートビートの表示用。</summary>
    public int MaxSessions => _maxSessions;

    /// <summary>これまでに生成したセッションの総数。</summary>
    public long TotalCreated => Interlocked.Read(ref _totalCreated);

    /// <summary>上限で拒否した接続の総数。</summary>
    public long TotalRejected => Interlocked.Read(ref _totalRejected);

    /// <summary>制御ループの異常終了で破棄したセッションの総数。</summary>
    public long TotalFaulted => Interlocked.Read(ref _totalFaulted);

    /// <summary>現在のセッション一覧の写し。ハートビートの表示用。</summary>
    public IReadOnlyList<ControllerSession> Snapshot() => _sessions.Values.ToArray();

    /// <summary>上限に達している場合は null を返す。呼び出し側は接続を拒否すること。</summary>
    public ControllerSession? Create(string connectionId, IClientProxy client)
    {
        if (_sessions.Count >= _maxSessions)
        {
            Interlocked.Increment(ref _totalRejected);
            _logger.LogWarning("セッション数の上限 {Max} に達したため接続を拒否しました {ConnectionId} (拒否累計 {Rejected})",
                _maxSessions, connectionId, _totalRejected);
            return null;
        }

        // ログのカテゴリがセッション自身になるよう、ファクトリごと渡す
        var session = new ControllerSession(connectionId, client, loggerFactory);
        if (!_sessions.TryAdd(connectionId, session))
        {
            // 同じ接続 ID が二重に来ることは無いが、来たら新しい方を捨てて既存を使う
            _logger.LogWarning("同じ接続 ID {ConnectionId} のセッションがすでに存在する。新しい方を破棄して既存を使う", connectionId);
            _ = session.DisposeAsync().AsTask();
            return _sessions.TryGetValue(connectionId, out var existing) ? existing : null;
        }

        session.Faulted += id =>
        {
            Interlocked.Increment(ref _totalFaulted);
            _logger.LogError("制御ループの異常終了によりセッション {ConnectionId} を破棄します (異常累計 {Faulted})", id, _totalFaulted);
            _ = RemoveAsync(id).AsTask();
        };
        long created = Interlocked.Increment(ref _totalCreated);
        _logger.LogInformation("セッション開始 {ConnectionId} (接続数 {Count}/{Max}, 生成累計 {Created})",
            connectionId, _sessions.Count, _maxSessions, created);
        return session;
    }

    public ControllerSession? Get(string connectionId)
        => _sessions.TryGetValue(connectionId, out var session) ? session : null;

    public async ValueTask RemoveAsync(string connectionId)
    {
        if (!_sessions.TryRemove(connectionId, out var session))
        {
            _logger.LogDebug("セッション {ConnectionId} はすでに削除済み (二重の切断通知か、異常終了と切断の競合)", connectionId);
            return;
        }
        _logger.LogDebug("セッション {ConnectionId} を一覧から外した。破棄を待つ (残り {Count})", connectionId, _sessions.Count);
        await session.DisposeAsync().ConfigureAwait(false);
        _logger.LogInformation("セッション終了 {ConnectionId} (接続数 {Count}/{Max})", connectionId, _sessions.Count, _maxSessions);
    }
}
