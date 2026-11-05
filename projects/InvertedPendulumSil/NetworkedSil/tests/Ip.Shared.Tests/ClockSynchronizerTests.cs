using Ip.Shared.Protocol;

namespace Ip.Shared.Tests;

/// <summary>
/// NTP 方式のクロック同期。同一マシンでの結合テストではオフセットがほぼ 0 になるため、
/// 符号反転や /2 落ちのようなバグが表に出ない。合成タイムスタンプで式そのものを固定する。
/// </summary>
public sealed class ClockSynchronizerTests
{
    /// <summary>
    /// サーバの時計がクライアントより <paramref name="offsetMs"/> だけ進んでいて、
    /// 上り <paramref name="upMs"/> / 下り <paramref name="downMs"/> の経路で往復した場合の 4 時刻を作る。
    /// </summary>
    private static (ClockSyncResult Result, double ClientReceiveMs) Exchange(
        double offsetMs, double upMs, double downMs, double clientSendMs = 1000.0, double serverProcessMs = 0.5)
    {
        double serverReceive = clientSendMs + upMs + offsetMs;
        double serverSend = serverReceive + serverProcessMs;
        double clientReceive = serverSend - offsetMs + downMs;
        return (new ClockSyncResult(clientSendMs, serverReceive, serverSend), clientReceive);
    }

    [Fact]
    public void Accept_RecoversASymmetricOffset()
    {
        var sync = new ClockSynchronizer();
        var (result, clientReceive) = Exchange(offsetMs: 1000.0, upMs: 5.0, downMs: 5.0);

        Assert.True(sync.Accept(result, clientReceive));

        Assert.Equal(1000.0, sync.OffsetMs, 6);
        Assert.Equal(10.0, sync.RoundTripMs, 6);
        Assert.Equal(1, sync.SampleCount);
    }

    [Fact]
    public void Accept_OnAnAsymmetricPath_BiasesByHalfTheImbalance()
    {
        // 上り 2ms / 下り 20ms。NTP 式は経路の対称性を仮定するので、差の半分だけ偏る。
        var sync = new ClockSynchronizer();
        var (result, clientReceive) = Exchange(offsetMs: 0.0, upMs: 2.0, downMs: 20.0);

        Assert.True(sync.Accept(result, clientReceive));

        Assert.Equal(-9.0, sync.OffsetMs, 6);   // (2 - 20) / 2
        Assert.Equal(22.0, sync.RoundTripMs, 6);
    }

    [Fact]
    public void Accept_KeepsTheSampleWithTheSmallestRoundTrip()
    {
        var sync = new ClockSynchronizer();
        var (slow, slowReceive) = Exchange(offsetMs: 500.0, upMs: 60.0, downMs: 60.0);
        var (fast, fastReceive) = Exchange(offsetMs: 500.0, upMs: 3.0, downMs: 3.0);

        Assert.True(sync.Accept(slow, slowReceive));
        Assert.True(sync.Accept(fast, fastReceive));
        Assert.Equal(6.0, sync.RoundTripMs, 6);

        // 遅いサンプルが後から来ても採用しない
        var (slowAgain, slowAgainReceive) = Exchange(offsetMs: 500.0, upMs: 80.0, downMs: 80.0);
        Assert.False(sync.Accept(slowAgain, slowAgainReceive));
        Assert.Equal(6.0, sync.RoundTripMs, 6);
        Assert.Equal(3, sync.SampleCount);
    }

    [Fact]
    public void Accept_RejectsImpossibleRoundTrips()
    {
        var sync = new ClockSynchronizer();
        // サーバの処理時間が往復時間より長い = 時計が単調でないか計測が壊れている
        var result = new ClockSyncResult(ClientSendMs: 1000.0, ServerReceiveMs: 1000.0, ServerSendMs: 1100.0);

        Assert.False(sync.Accept(result, clientReceiveMs: 1050.0));
        Assert.Equal(0.0, sync.OffsetMs);
    }

    [Fact]
    public void Reset_ClearsTheEstimate()
    {
        var sync = new ClockSynchronizer();
        var (result, clientReceive) = Exchange(offsetMs: 1000.0, upMs: 5.0, downMs: 5.0);
        sync.Accept(result, clientReceive);

        sync.Reset();

        Assert.Equal(0.0, sync.OffsetMs);
        Assert.Equal(0.0, sync.RoundTripMs);
        Assert.Equal(0, sync.SampleCount);
    }
}
