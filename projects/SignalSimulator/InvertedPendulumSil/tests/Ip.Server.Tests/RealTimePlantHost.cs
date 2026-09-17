using System.Collections.Concurrent;
using Ip.Shared;
using Ip.Shared.Protocol;
using Ip.Shared.Simulation;
using Microsoft.AspNetCore.SignalR.Client;

namespace Ip.Server.Tests;

/// <summary>
/// テスト用のプラントホスト。Blazor クライアントの <c>PlantRunner</c> と同じ役割を、
/// ブラウザなしで実時間のまま果たす。制御ロジックは持たない。
/// </summary>
public sealed class RealTimePlantHost : IAsyncDisposable
{
    private readonly HubConnection _connection;
    private readonly ConcurrentQueue<object> _pending = new();
    private readonly CancellationTokenSource _cts = new();
    private readonly Task _loop;
    private readonly List<IDisposable> _registrations = [];

    public RealTimePlantHost(HubConnection connection)
    {
        _connection = connection;
        Plant = new VirtualPlant(random: new Random(7));
        Plant.FeedbackReady += feedback => _pending.Enqueue(feedback);
        Plant.MotionEventRaised += motionEvent => _pending.Enqueue(motionEvent);

        _registrations.Add(connection.On<VoltageCommand>(
            HubMethods.OnVoltage, command => Plant.ApplyCommand(command, MonotonicClock.NowMs)));
        _registrations.Add(connection.On<AbortCommand>(HubMethods.OnAbort, Plant.ApplyAbort));

        _loop = Task.Run(() => RunAsync(_cts.Token));
    }

    public VirtualPlant Plant { get; }

    private async Task RunAsync(CancellationToken token)
    {
        try
        {
            while (!token.IsCancellationRequested)
            {
                Plant.AdvanceTo(MonotonicClock.NowMs);

                while (_pending.TryDequeue(out var message))
                {
                    string method = message is EncoderFeedback ? HubMethods.SendFeedback : HubMethods.SendMotionEvent;
                    await _connection.SendAsync(method, message, token).ConfigureAwait(false);
                }

                await Task.Delay(1, token).ConfigureAwait(false);
            }
        }
        catch (OperationCanceledException)
        {
        }
    }

    public async ValueTask DisposeAsync()
    {
        await _cts.CancelAsync().ConfigureAwait(false);
        try { await _loop.ConfigureAwait(false); } catch (OperationCanceledException) { }
        foreach (var registration in _registrations) registration.Dispose();
        _cts.Dispose();
    }
}
