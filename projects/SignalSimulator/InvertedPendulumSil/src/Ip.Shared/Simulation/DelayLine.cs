using Ip.Shared.Protocol;

namespace Ip.Shared.Simulation;

/// <summary>
/// 通信路エミュレータ。SignalR は WebSocket (TCP) 上で動くのでパケットは欠落せず順序も保たれる。
/// したがって損失は「再送による遅延スパイク」として現れる —— これを次のように模擬する。
///
/// <list type="bullet">
///   <item>片道遅延 + 一様ジッタ</item>
///   <item>確率 <see cref="NetworkConfig.SpikePercent"/> で +150ms のスパイク (TCP 再送相当)</item>
///   <item>追い越し禁止 (前のメッセージより早く着かない = HOL ブロッキング)</item>
/// </list>
///
/// 遅延の基準は送信処理の時刻ではなく、呼び出し側が渡す「計測時刻」である。
/// プラントは 1ms ステップをまとめて進めるため、送信処理の時刻を使うと遅延がステップ境界に量子化されてしまう。
/// </summary>
public sealed class DelayLine<T>(Action<T> deliver, Random? random = null)
{
    private const double SpikeDelayMs = 150.0;

    private readonly Action<T> _deliver = deliver ?? throw new ArgumentNullException(nameof(deliver));
    private readonly Random _random = random ?? Random.Shared;
    private readonly Queue<(double At, T Message)> _queue = new();
    private readonly Lock _gate = new();
    private double _lastDeliveryAt;

    public int Count
    {
        get { lock (_gate) return _queue.Count; }
    }

    /// <summary>メッセージを投入する。<paramref name="stampMs"/> は遅延の起点となる時刻。</summary>
    public void Push(T message, double stampMs, NetworkConfig config)
    {
        double delay = config.LatencyMs + _random.NextDouble() * config.JitterMs;
        if (_random.NextDouble() * 100.0 < config.SpikePercent) delay += SpikeDelayMs;

        lock (_gate)
        {
            double at = Math.Max(_lastDeliveryAt, stampMs + delay);
            _lastDeliveryAt = at;
            _queue.Enqueue((at, message));
        }
    }

    /// <summary>遅延なしで即座に配送する (通信路エミュレーションを使わない構成用)。</summary>
    public void PushImmediate(T message) => _deliver(message);

    /// <summary>到達時刻を過ぎたメッセージを配送する。呼び出し側が定期的に呼ぶ。</summary>
    public void Flush(double nowMs)
    {
        while (true)
        {
            T message;
            lock (_gate)
            {
                if (_queue.Count == 0 || _queue.Peek().At > nowMs) return;
                message = _queue.Dequeue().Message;
            }
            _deliver(message);
        }
    }

    public void Clear()
    {
        lock (_gate)
        {
            _queue.Clear();
            _lastDeliveryAt = 0.0;
        }
    }
}
