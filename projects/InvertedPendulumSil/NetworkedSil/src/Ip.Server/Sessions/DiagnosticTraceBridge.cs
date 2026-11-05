using Ip.Shared.Diagnostics;

namespace Ip.Server.Sessions;

/// <summary>
/// <c>Ip.Shared</c> の診断トレースを ASP.NET Core の <see cref="ILogger"/> へ橋渡しする。
///
/// トレースの発生源ごとに独立したログカテゴリ (<c>Ip.Controller</c> など) を割り当てるので、
/// <c>appsettings.json</c> の <c>Logging:LogLevel</c> で「コントローラの帰還 1 本ごとの内容だけ Trace で見る」
/// といった絞り込みができる。ロガー側で無効な重要度はトレース側の <see cref="DiagnosticTrace.MinimumLevel"/> にも反映し、
/// 制御ループの内側で文字列を整形しないようにする。
/// </summary>
public static class DiagnosticTraceBridge
{
    /// <summary>ログカテゴリの接頭辞。<c>Ip.Controller</c>, <c>Ip.Plant</c>, <c>Ip.Uplink</c>, <c>Ip.Downlink</c> になる。</summary>
    public const string CategoryPrefix = "Ip.";

    /// <param name="trace">橋渡しするトレース</param>
    /// <param name="loggerFactory">カテゴリ付きロガーを作るファクトリ</param>
    /// <param name="context">全行に付ける文脈 (接続 ID など)。null なら付けない。</param>
    /// <returns>橋渡しに使ったロガー。ホスト側の追加ログにも使える。</returns>
    public static ILogger Attach(DiagnosticTrace trace, ILoggerFactory loggerFactory, string? context = null)
    {
        var logger = loggerFactory.CreateLogger(CategoryPrefix + Capitalize(trace.Source));
        trace.MinimumLevel = MinimumEnabledLevel(logger);
        trace.Emitted += e =>
        {
            if (context is null) logger.Log(ToLogLevel(e.Level), "{Message}", e.Message);
            else logger.Log(ToLogLevel(e.Level), "[{Context}] {Message}", context, e.Message);
        };
        return logger;
    }

    public static LogLevel ToLogLevel(DiagnosticLevel level) => level switch
    {
        DiagnosticLevel.Trace => LogLevel.Trace,
        DiagnosticLevel.Debug => LogLevel.Debug,
        DiagnosticLevel.Information => LogLevel.Information,
        DiagnosticLevel.Warning => LogLevel.Warning,
        _ => LogLevel.Error,
    };

    /// <summary>ロガーで有効な最も低い重要度。全部無効なら Error より上 (= 何も出さない)。</summary>
    public static DiagnosticLevel MinimumEnabledLevel(ILogger logger)
    {
        foreach (var level in new[]
                 {
                     DiagnosticLevel.Trace, DiagnosticLevel.Debug, DiagnosticLevel.Information,
                     DiagnosticLevel.Warning, DiagnosticLevel.Error,
                 })
        {
            if (logger.IsEnabled(ToLogLevel(level))) return level;
        }
        return (DiagnosticLevel)((int)DiagnosticLevel.Error + 1);
    }

    private static string Capitalize(string s)
        => s.Length == 0 ? s : char.ToUpperInvariant(s[0]) + s[1..];
}
