using System.Globalization;
using System.Runtime.CompilerServices;

namespace Ip.Shared.Diagnostics;

/// <summary>診断トレースの重要度。<c>Microsoft.Extensions.Logging.LogLevel</c> と同じ順序に揃えている。</summary>
public enum DiagnosticLevel
{
    /// <summary>帰還 1 本・指令 1 本ごとの内容。数百行/秒になるので既定では出さない。</summary>
    Trace = 0,
    /// <summary>1 秒ごとの集計、破棄した指令、キューの状態など。</summary>
    Debug = 1,
    /// <summary>状態遷移・設定変更・再設計など、動作の節目。</summary>
    Information = 2,
    /// <summary>異常ではないが注意を要する事象 (時間スリップ、旧セッションの指令など)。</summary>
    Warning = 3,
    /// <summary>FAULT、保護動作、ループの異常終了。</summary>
    Error = 4,
}

/// <summary>診断トレース 1 件。</summary>
/// <param name="Level">重要度</param>
/// <param name="Source">発生源 (controller / plant / uplink / downlink)</param>
/// <param name="Message">本文</param>
public readonly record struct DiagnosticEvent(DiagnosticLevel Level, string Source, string Message);

/// <summary>
/// 「内部で何が起きているか」を高頻度で外へ出すための出口。
///
/// UI に出す <see cref="Protocol.LogEntry"/> はオペレータ向けの少量のログで、
/// 帰還 1 本ごとの内容や 1 秒ごとの集計まで流すと画面が埋まる。
/// そこで診断用の経路を分け、ホスト側 (ASP.NET Core の <c>ILogger</c>、ブラウザの console) へ橋渡しする。
///
/// <c>Ip.Shared</c> は WASM とテストでも同じコードで動くため、ロギング基盤には依存しない。
/// 購読者がいない、または <see cref="MinimumLevel"/> 未満のときは
/// 補間文字列の整形すら行わない (<see cref="DiagnosticInterpolatedStringHandler"/>)。
/// 制御ループの内側から呼んでもコストが乗らないようにするため。
/// </summary>
public sealed class DiagnosticTrace(string source)
{
    public string Source { get; } = source;

    /// <summary>これ未満の重要度は捨てる。ホスト側のログレベルに合わせて設定する。</summary>
    public DiagnosticLevel MinimumLevel { get; set; } = DiagnosticLevel.Trace;

    /// <summary>トレースの購読。ホスト側でログ基盤へつなぐ。</summary>
    public event Action<DiagnosticEvent>? Emitted;

    public bool IsEnabled(DiagnosticLevel level) => Emitted is not null && level >= MinimumLevel;

    /// <summary>補間文字列で書く。無効な重要度なら整形されない。</summary>
    public void Write(
        DiagnosticLevel level,
        [InterpolatedStringHandlerArgument("", nameof(level))] ref DiagnosticInterpolatedStringHandler message)
    {
        if (!IsEnabled(level)) return;
        Emitted?.Invoke(new DiagnosticEvent(level, Source, message.ToStringAndClear()));
    }

    /// <summary>定数文字列で書く。</summary>
    public void Write(DiagnosticLevel level, string message)
    {
        if (!IsEnabled(level)) return;
        Emitted?.Invoke(new DiagnosticEvent(level, Source, message));
    }
}

/// <summary>
/// <see cref="DiagnosticTrace.Write(DiagnosticLevel, ref DiagnosticInterpolatedStringHandler)"/> 用の補間文字列ハンドラ。
/// トレースが無効なら <c>shouldAppend</c> を false にして、補間の各項の評価と整形をコンパイラに省かせる。
/// </summary>
[InterpolatedStringHandler]
public ref struct DiagnosticInterpolatedStringHandler
{
    private DefaultInterpolatedStringHandler _inner;
    private readonly bool _enabled;

    public DiagnosticInterpolatedStringHandler(
        int literalLength, int formattedCount, DiagnosticTrace trace, DiagnosticLevel level, out bool shouldAppend)
    {
        _enabled = shouldAppend = trace.IsEnabled(level);
        _inner = _enabled
            ? new DefaultInterpolatedStringHandler(literalLength, formattedCount, CultureInfo.InvariantCulture)
            : default;
    }

    public void AppendLiteral(string value) => _inner.AppendLiteral(value);
    public void AppendFormatted<T>(T value) => _inner.AppendFormatted(value);
    public void AppendFormatted<T>(T value, string? format) => _inner.AppendFormatted(value, format);
    public void AppendFormatted<T>(T value, int alignment) => _inner.AppendFormatted(value, alignment);
    public void AppendFormatted<T>(T value, int alignment, string? format) => _inner.AppendFormatted(value, alignment, format);
    public void AppendFormatted(ReadOnlySpan<char> value) => _inner.AppendFormatted(value);
    public void AppendFormatted(string? value) => _inner.AppendFormatted(value);

    public string ToStringAndClear() => _enabled ? _inner.ToStringAndClear() : string.Empty;
}
