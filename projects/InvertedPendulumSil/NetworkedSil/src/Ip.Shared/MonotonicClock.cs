using System.Diagnostics;

namespace Ip.Shared;

/// <summary>
/// プロセス起動を原点とする単調増加のミリ秒時計。
///
/// 制御ループの時刻基準に <see cref="DateTime"/> を使うと NTP 補正で時刻が巻き戻り、
/// 遅延の計算が壊れる。Stopwatch を基準にすることでそれを避ける。
/// プロセス間で原点が違うことは前提で、その差は <see cref="Protocol.ClockSynchronizer"/> が吸収する。
/// </summary>
public static class MonotonicClock
{
    private static readonly Stopwatch Watch = Stopwatch.StartNew();

    public static double NowMs => Watch.Elapsed.TotalMilliseconds;
}
