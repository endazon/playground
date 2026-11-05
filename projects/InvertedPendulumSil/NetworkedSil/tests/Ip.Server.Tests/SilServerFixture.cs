using Ip.Shared.Protocol;
using Microsoft.AspNetCore.Mvc.Testing;
using Microsoft.AspNetCore.SignalR.Client;
using Microsoft.AspNetCore.TestHost;
using Microsoft.Extensions.DependencyInjection;

namespace Ip.Server.Tests;

/// <summary>
/// 実際の <c>Ip.Server</c> を TestServer 上で起動し、本物の SignalR クライアントで接続する。
/// MessagePack のシリアライズ、Hub のメソッド束縛、セッションの制御ループまで、
/// 単体テストでは通らない経路をまとめて通す。
/// </summary>
public sealed class SilServerFixture : WebApplicationFactory<Program>
{
    /// <summary>TestServer 経由で Hub に接続する。WebSocket も TestServer のものを使う。</summary>
    public HubConnection CreateHubConnection()
    {
        var server = Server; // 遅延初期化されるので先に触れておく

        return new HubConnectionBuilder()
            .WithUrl(new Uri(server.BaseAddress, "hub/sil"), options =>
            {
                options.HttpMessageHandlerFactory = _ => server.CreateHandler();
                options.WebSocketFactory = async (context, token) =>
                {
                    var client = server.CreateWebSocketClient();
                    var uri = new UriBuilder(context.Uri) { Scheme = "http" }.Uri;
                    return await client.ConnectAsync(uri, token).ConfigureAwait(false);
                };
            })
            .AddMessagePackProtocol()
            .Build();
    }
}

/// <summary>受信したメッセージを溜めておく、テスト用のクライアント側受け皿。</summary>
public sealed class HubInbox : IAsyncDisposable
{
    private readonly HubConnection _connection;
    private readonly List<IDisposable> _registrations = [];

    public HubInbox(HubConnection connection)
    {
        _connection = connection;
        _registrations.Add(connection.On<VoltageCommand>(HubMethods.OnVoltage, m => { lock (Gate) Voltages.Add(m); }));
        _registrations.Add(connection.On<AbortCommand>(HubMethods.OnAbort, m => { lock (Gate) Aborts.Add(m); }));
        _registrations.Add(connection.On<DesignInfo>(HubMethods.OnDesign, m => { lock (Gate) Designs.Add(m); }));
        _registrations.Add(connection.On<ControllerStatus>(HubMethods.OnStatus, m => { lock (Gate) Statuses.Add(m); }));
        _registrations.Add(connection.On<LogEntry>(HubMethods.OnLog, m => { lock (Gate) Logs.Add(m); }));
    }

    public Lock Gate { get; } = new();
    public List<VoltageCommand> Voltages { get; } = [];
    public List<AbortCommand> Aborts { get; } = [];
    public List<DesignInfo> Designs { get; } = [];
    public List<ControllerStatus> Statuses { get; } = [];
    public List<LogEntry> Logs { get; } = [];

    public T? Latest<T>(List<T> source)
    {
        lock (Gate) return source.Count == 0 ? default : source[^1];
    }

    public int CountOf<T>(List<T> source)
    {
        lock (Gate) return source.Count;
    }

    /// <summary>条件が満たされるまで待つ。満たされなければ false。</summary>
    public async Task<bool> WaitUntilAsync(Func<bool> predicate, TimeSpan timeout)
    {
        var deadline = DateTime.UtcNow + timeout;
        while (DateTime.UtcNow < deadline)
        {
            if (predicate()) return true;
            await Task.Delay(10).ConfigureAwait(false);
        }
        return predicate();
    }

    public async ValueTask DisposeAsync()
    {
        foreach (var registration in _registrations) registration.Dispose();
        await _connection.DisposeAsync().ConfigureAwait(false);
    }
}
