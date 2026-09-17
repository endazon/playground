using Ip.Shared.Protocol;

namespace Ip.Client.Services;

/// <summary>画面のイベントログ 1 行。発生源 (バック / プラント / システム) を必ず残す。</summary>
/// <param name="ElapsedSeconds">アプリ起動からの経過秒</param>
/// <param name="Source">発生源</param>
/// <param name="Level">重要度</param>
/// <param name="Message">本文</param>
public sealed record UiLogEntry(double ElapsedSeconds, string Source, LogSeverity Level, string Message)
{
    public string CssClass => Level switch
    {
        LogSeverity.Warning => "warn",
        LogSeverity.Error => "error",
        _ => "info",
    };
}

/// <summary>プラント (ローカル) 側のテレメトリ。UI の指標表示に使う。</summary>
public sealed record PlantTelemetry(
    double SimSeconds,
    double X,
    double Theta,
    double XDot,
    double ThetaDot,
    double Volts,
    bool DriveEnabled,
    bool EmergencyStopped,
    double E2EMs,
    long RejectedCommands,
    long AppliedCommands,
    int TimeSlips,
    int UplinkQueueLength)
{
    public static readonly PlantTelemetry Empty =
        new(0, 0, Math.PI, 0, 0, 0, true, false, 0, 0, 0, 0, 0);
}

/// <summary>接続状態。</summary>
public enum LinkState
{
    Connecting,
    Connected,
    Reconnecting,
    Disconnected,
}
