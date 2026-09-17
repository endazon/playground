using Ip.Shared.Diagnostics;
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
///
/// 診断トレース <see cref="Trace"/> を与えると、スパイクの注入 (Debug) と
/// 投入・配送 1 件ごとの遅延 (Trace) を出す。配送の「遅れ」 (到達時刻と実際に配送した時刻の差) は
/// <see cref="Flush"/> を呼ぶループの周期粒度がそのまま現れる量で、タイマの粗さを見るのに役立つ。
/// </summary>
public sealed class DelayLine<T>(Action<T> deliver, Random? random = null)
{
    private const double SpikeDelayMs = 150.0;

    private readonly Action<T> _deliver = deliver ?? throw new ArgumentNullException(nameof(deliver));
    private readonly Random _random = random ?? Random.Shared;
    private readonly Queue<(double At, T Message)> _queue = new();
    private readonly Lock _gate = new();
    private double _lastDeliveryAt;
    private long _pushed;
    private long _delivered;
    private long _spikes;
    private long _headOfLineBlocked;
    private int _maxQueueLength;
    private double _maxLatenessMs;

    /// <summary>診断トレース。null なら何も出さない。</summary>
    public DiagnosticTrace? Trace { get; init; }

    public int Count
    {
        get { lock (_gate) return _queue.Count; }
    }

    /// <summary>投入した件数。</summary>
    public long PushedCount => Interlocked.Read(ref _pushed);
    /// <summary>配送した件数。</summary>
    public long DeliveredCount => Interlocked.Read(ref _delivered);
    /// <summary>再送スパイクを注入した件数。</summary>
    public long SpikeCount => Interlocked.Read(ref _spikes);
    /// <summary>追い越し禁止のため、前のメッセージの到達時刻まで待たされた件数。</summary>
    public long HeadOfLineBlockedCount => Interlocked.Read(ref _headOfLineBlocked);
    /// <summary>これまでの最大キュー長。</summary>
    public int MaxQueueLength => Volatile.Read(ref _maxQueueLength);
    /// <summary>到達時刻から実際の配送までの遅れの最大値 [ms]。Flush の呼び出し粒度が現れる。</summary>
    public double MaxLatenessMs => Volatile.Read(ref _maxLatenessMs);

    /// <summary>メッセージを投入する。<paramref name="stampMs"/> は遅延の起点となる時刻。</summary>
    public void Push(T message, double stampMs, NetworkConfig config)
    {
        double delay = config.LatencyMs + _random.NextDouble() * config.JitterMs;
        bool spiked = _random.NextDouble() * 100.0 < config.SpikePercent;
        if (spiked) delay += SpikeDelayMs;

        double at;
        bool blocked;
        int queueLength;
        lock (_gate)
        {
            at = Math.Max(_lastDeliveryAt, stampMs + delay);
            blocked = at > stampMs + delay;
            _lastDeliveryAt = at;
            _queue.Enqueue((at, message));
            queueLength = _queue.Count;
        }

        Interlocked.Increment(ref _pushed);
        if (spiked) Interlocked.Increment(ref _spikes);
        if (blocked) Interlocked.Increment(ref _headOfLineBlocked);
        if (queueLength > _maxQueueLength) Volatile.Write(ref _maxQueueLength, queueLength);

        if (Trace is null) return;
        if (spiked)
        {
            Trace.Write(DiagnosticLevel.Debug,
                $"再送スパイクを注入: +{SpikeDelayMs:0} ms → {typeof(T).Name} は {at - stampMs:0.0} ms 後に到達 (キュー {queueLength} 件, 累計スパイク {_spikes})");
        }
        if (Trace.IsEnabled(DiagnosticLevel.Trace))
        {
            Trace.Write(DiagnosticLevel.Trace,
                $"投入 {Describe(message)}: 起点 {stampMs:0.0} + 遅延 {delay:0.0} ms → 到達 {at:0.0}{(blocked ? $" (追い越し禁止で +{at - stampMs - delay:0.0} ms)" : "")}, キュー {queueLength} 件");
        }
    }

    /// <summary>
    /// 到達時刻を過ぎたメッセージを配送する。呼び出し側が定期的に呼ぶ。
    ///
    /// <see cref="Push"/> は <see cref="Flush"/> と並行に呼んでよい (投入順に追い越しは起きない) が、
    /// <see cref="Flush"/> 自体は<b>単一スレッドから呼ぶこと</b>。
    /// 配送コールバックをロックの外で呼ぶため、2 スレッドから同時に流すと追い越しが起きる。
    /// </summary>
    public void Flush(double nowMs)
    {
        while (true)
        {
            T message;
            double at;
            int remaining;
            lock (_gate)
            {
                if (_queue.Count == 0 || _queue.Peek().At > nowMs) return;
                (at, message) = _queue.Dequeue();
                remaining = _queue.Count;
            }

            Interlocked.Increment(ref _delivered);
            double lateness = nowMs - at;
            if (lateness > _maxLatenessMs) Volatile.Write(ref _maxLatenessMs, lateness);

            if (Trace is not null && Trace.IsEnabled(DiagnosticLevel.Trace))
            {
                Trace.Write(DiagnosticLevel.Trace,
                    $"配送 {Describe(message)}: 到達予定 {at:0.0} → 配送 {nowMs:0.0} (遅れ {lateness:0.0} ms), 残り {remaining} 件");
            }

            _deliver(message);
        }
    }

    /// <summary>集計値を 1 行にまとめる。ホスト側の定期ログ用。</summary>
    public string DescribeStatistics()
        => $"投入 {PushedCount} / 配送 {DeliveredCount} / 滞留 {Count} 件, スパイク {SpikeCount}, 追い越し待ち {HeadOfLineBlockedCount}, 最大キュー {MaxQueueLength} 件, 配送遅れ最大 {MaxLatenessMs:0.0} ms";

    private static string Describe(T message) => message switch
    {
        EncoderFeedback f => $"EncoderFeedback Seq {f.Seq}",
        MotionEvent e => $"MotionEvent {e.Kind}",
        VoltageCommand c => $"VoltageCommand #{c.CommandId}/{c.Seq}",
        AbortCommand a => $"AbortCommand #{a.CommandId}",
        null => "null",
        _ => message.GetType().Name,
    };
}
