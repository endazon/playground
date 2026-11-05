using Ip.Shared.Control;
using Ip.Shared.Model;
using Ip.Shared.Protocol;
using Microsoft.AspNetCore.SignalR.Client;

namespace Ip.Server.Tests;

/// <summary>
/// SignalR + MessagePack を実際に通した結合試験。仮想時間ではなく実時間で回るので、
/// 「サーバの制御ループが本当に 5ms 周期に追随できるか」もここで分かる。
/// </summary>
public sealed class SilHubTests(SilServerFixture fixture) : IClassFixture<SilServerFixture>
{
    private static readonly TimeSpan Timeout = TimeSpan.FromSeconds(10);

    private async Task<(HubConnection Connection, HubInbox Inbox)> ConnectAsync()
    {
        var connection = fixture.CreateHubConnection();
        var inbox = new HubInbox(connection);
        await connection.StartAsync();
        return (connection, inbox);
    }

    [Fact]
    public async Task Connecting_DeliversTheCurrentDesign()
    {
        var (connection, inbox) = await ConnectAsync();
        await using var _ = inbox;

        Assert.True(await inbox.WaitUntilAsync(() => inbox.CountOf(inbox.Designs) > 0, Timeout));

        var design = inbox.Latest(inbox.Designs)!;
        Assert.Equal(4, design.Gains.Length);
        Assert.Equal(5, design.PeriodMs);
        Assert.InRange(design.UnstablePole, 5.0, 5.6);
        Assert.InRange(design.DelayMarginMs, 80, 90);
        await connection.StopAsync();
    }

    [Fact]
    public async Task ChangingThePeriod_TriggersARedesign()
    {
        var (connection, inbox) = await ConnectAsync();
        await using var _ = inbox;
        Assert.True(await inbox.WaitUntilAsync(() => inbox.CountOf(inbox.Designs) > 0, Timeout));

        await connection.SendAsync(HubMethods.ConfigureNetwork, new NetworkConfig(PeriodMs: 100));

        Assert.True(await inbox.WaitUntilAsync(
            () => inbox.Latest(inbox.Designs)?.PeriodMs == 100, Timeout));

        // 周期を伸ばすと理論遅延余裕は縮む (設計メモの表と同じ傾向)
        Assert.InRange(inbox.Latest(inbox.Designs)!.DelayMarginMs, 55, 63);
        await connection.StopAsync();
    }

    [Fact]
    public async Task SyncClock_ReturnsOrderedTimestamps()
    {
        var (connection, inbox) = await ConnectAsync();
        await using var _ = inbox;

        double sendAt = Ip.Shared.MonotonicClock.NowMs;
        var result = await connection.InvokeAsync<ClockSyncResult>(HubMethods.SyncClock, sendAt);

        Assert.Equal(sendAt, result.ClientSendMs, 6);
        Assert.True(result.ServerSendMs >= result.ServerReceiveMs);

        var synchronizer = new ClockSynchronizer();
        Assert.True(synchronizer.Accept(result, Ip.Shared.MonotonicClock.NowMs));
        Assert.True(synchronizer.RoundTripMs >= 0.0);
        await connection.StopAsync();
    }

    [Fact]
    public async Task Disconnecting_ReleasesTheSession()
    {
        using var http = fixture.CreateClient();

        var (connection, inbox) = await ConnectAsync();
        await using var _ = inbox;
        Assert.True(await inbox.WaitUntilAsync(() => inbox.CountOf(inbox.Designs) > 0, Timeout));

        int whileConnected = await ReadSessionCountAsync(http);
        Assert.True(whileConnected >= 1, $"接続中なのにセッションが無い: {whileConnected}");

        await connection.StopAsync();

        // OnDisconnectedAsync → RemoveAsync → DisposeAsync が回ってセッションが解放されること
        var deadline = DateTime.UtcNow + Timeout;
        while (DateTime.UtcNow < deadline && await ReadSessionCountAsync(http) >= whileConnected)
            await Task.Delay(50);

        Assert.True(await ReadSessionCountAsync(http) < whileConnected, "切断してもセッションが解放されない");
    }

    [Fact]
    public async Task TwoConnections_GetIndependentControllers()
    {
        var (first, firstInbox) = await ConnectAsync();
        await using var _ = firstInbox;
        await using var firstHost = new RealTimePlantHost(first);

        var (second, secondInbox) = await ConnectAsync();
        await using var __ = secondInbox;

        await first.SendAsync(HubMethods.ConfigureNetwork, new NetworkConfig(PeriodMs: 5));
        firstHost.Plant.Reset(upright: true);
        await Task.Delay(100);
        await first.SendAsync(HubMethods.Operate, OperatorAction.Balance);

        Assert.True(await firstInbox.WaitUntilAsync(
            () => firstInbox.Latest(firstInbox.Statuses)?.Mode == ControlMode.Balance, Timeout));

        // 2 本目は何も操作していないので IDLE のまま。片方の運転が他方に漏れない。
        Assert.True(await secondInbox.WaitUntilAsync(() => secondInbox.CountOf(secondInbox.Designs) > 0, Timeout));
        var secondStatus = secondInbox.Latest(secondInbox.Statuses);
        Assert.True(secondStatus is null or { Mode: ControlMode.Idle },
            $"2 本目のセッションが巻き込まれている: {secondStatus?.Mode}");
        Assert.Empty(secondInbox.Voltages);

        await first.StopAsync();
        await second.StopAsync();
    }

    private static async Task<int> ReadSessionCountAsync(HttpClient http)
    {
        using var document = System.Text.Json.JsonDocument.Parse(await http.GetStringAsync("/healthz"));
        return document.RootElement.GetProperty("sessions").GetInt32();
    }

    [Fact]
    public async Task Balance_OverTheWire_KeepsThePendulumUpright()
    {
        var (connection, inbox) = await ConnectAsync();
        await using var _ = inbox;
        await using var host = new RealTimePlantHost(connection);

        await connection.SendAsync(HubMethods.ConfigureNetwork, new NetworkConfig(PeriodMs: 5));
        await connection.SendAsync(HubMethods.ConfigureControl, new ControlOptions());
        host.Plant.Reset(upright: true);
        await Task.Delay(100);

        await connection.SendAsync(HubMethods.Operate, OperatorAction.Balance);

        Assert.True(await inbox.WaitUntilAsync(
            () => inbox.Latest(inbox.Statuses)?.Mode == ControlMode.Balance, Timeout),
            "BALANCE に入らなかった");

        // 外乱を入れながら 4 秒維持させる
        for (int i = 0; i < 4; i++)
        {
            await Task.Delay(1000);
            host.Plant.Push(i % 2 == 0 ? 1 : -1);
            Assert.True(Math.Abs(CartPoleDynamics.WrapAngle(host.Plant.State.Theta)) < ControllerCore.FallAngleRad,
                $"{i + 1} 秒目で転倒した");
        }

        var status = inbox.Latest(inbox.Statuses)!;
        Assert.Equal(ControlMode.Balance, status.Mode);
        Assert.True(status.FeedbackCount > 300, $"帰還が届いていない: {status.FeedbackCount}");
        Assert.InRange(status.FeedbackIntervalMs, 3.0, 9.0);
        Assert.True(host.Plant.AppliedCommands > 300, $"指令が適用されていない: {host.Plant.AppliedCommands}");
        await connection.StopAsync();
    }

    [Fact]
    public async Task EmergencyStop_OverTheWire_FaultsTheController()
    {
        var (connection, inbox) = await ConnectAsync();
        await using var _ = inbox;
        await using var host = new RealTimePlantHost(connection);

        await connection.SendAsync(HubMethods.ConfigureNetwork, new NetworkConfig(PeriodMs: 5));
        host.Plant.Reset(upright: true);
        await Task.Delay(100);
        await connection.SendAsync(HubMethods.Operate, OperatorAction.Balance);
        Assert.True(await inbox.WaitUntilAsync(
            () => inbox.Latest(inbox.Statuses)?.Mode == ControlMode.Balance, Timeout));

        host.Plant.EmergencyStop(Ip.Shared.MonotonicClock.NowMs);

        Assert.True(await inbox.WaitUntilAsync(
            () => inbox.Latest(inbox.Statuses)?.Mode == ControlMode.Fault, Timeout),
            "非常停止がバックに伝わらなかった");
        Assert.Equal(0.0, host.Plant.AppliedVolts);
        Assert.True(await inbox.WaitUntilAsync(() => inbox.CountOf(inbox.Aborts) > 0, Timeout),
            "AbortCommand が送られてこなかった");
        await connection.StopAsync();
    }

    [Fact]
    public async Task InjectedLatency_ShowsUpInTheMeasuredEndToEndDelay()
    {
        var (connection, inbox) = await ConnectAsync();
        await using var _ = inbox;
        await using var host = new RealTimePlantHost(connection);

        // 下り側だけ 25ms の遅延を注入する (上りはこのテストホストでは素通し)
        await connection.SendAsync(HubMethods.ConfigureNetwork, new NetworkConfig(PeriodMs: 5, LatencyMs: 25));
        host.Plant.Reset(upright: true);
        await Task.Delay(100);
        await connection.SendAsync(HubMethods.Operate, OperatorAction.Balance);

        Assert.True(await inbox.WaitUntilAsync(
            () => inbox.Latest(inbox.Statuses)?.E2EDelayMs > 20.0, Timeout),
            "注入した遅延が実測 E2E 遅延に現れなかった");

        await Task.Delay(1500);
        Assert.True(Math.Abs(CartPoleDynamics.WrapAngle(host.Plant.State.Theta)) < ControllerCore.FallAngleRad,
            "遅延余裕の内側なのに倒れた");
        await connection.StopAsync();
    }
}
