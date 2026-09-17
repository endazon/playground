namespace Ip.Shared.Protocol;

/// <summary>
/// NTP 方式のクロック同期。プラント (ブラウザ) とコントローラ (サーバ) は別の時計で動くため、
/// 「上り遅延 = 受信時刻 - 計測時刻」をそのまま計算すると時計のズレが丸ごと乗ってしまう。
///
/// 往復の 4 時刻から
/// <c>offset = ((Rs - Sc) + (Ss - Rc)) / 2</c>、<c>rtt = (Rc - Sc) - (Ss - Rs)</c>
/// を求め、RTT が最小だったサンプルのオフセットを採用する (経路が最も空いていたときの推定が最も正確)。
/// </summary>
public sealed class ClockSynchronizer
{
    private double _bestRtt = double.PositiveInfinity;

    /// <summary>プラント時刻 → コントローラ時刻のオフセット [ms] (controller = plant + offset)。</summary>
    public double OffsetMs { get; private set; }

    /// <summary>採用したサンプルの往復時間 [ms]。</summary>
    public double RoundTripMs { get; private set; }

    public int SampleCount { get; private set; }

    /// <param name="result">サーバが返した受信・送信時刻</param>
    /// <param name="clientReceiveMs">クライアントが応答を受け取った時刻</param>
    /// <returns>推定を更新したら true</returns>
    public bool Accept(ClockSyncResult result, double clientReceiveMs)
    {
        double rtt = (clientReceiveMs - result.ClientSendMs) - (result.ServerSendMs - result.ServerReceiveMs);
        if (rtt < 0.0) return false;

        SampleCount++;
        if (rtt > _bestRtt) return false;

        _bestRtt = rtt;
        RoundTripMs = rtt;
        OffsetMs = ((result.ServerReceiveMs - result.ClientSendMs) + (result.ServerSendMs - clientReceiveMs)) / 2.0;
        return true;
    }

    public void Reset()
    {
        _bestRtt = double.PositiveInfinity;
        OffsetMs = 0.0;
        RoundTripMs = 0.0;
        SampleCount = 0;
    }
}
