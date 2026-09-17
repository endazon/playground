using MessagePack;

namespace Ip.Shared.Protocol;

/// <summary>
/// 下り (コントローラ → プラント) の電圧指令。帰還を 1 つ受け取って制御計算するたびに 1 つ送る。
/// </summary>
/// <param name="CommandId">運転セッション番号。開始操作のたびに増える。古い番号の指令はプラントが破棄する。</param>
/// <param name="Seq">セッション内の連番。逆行した指令はプラントが破棄する。</param>
/// <param name="Volts">指令電圧 [V]</param>
/// <param name="BasedOnMs">この指令の根拠にした計測時刻 (プラント時刻系) [ms]。E2E 遅延の実測に使う。</param>
[MessagePackObject]
public sealed record VoltageCommand(
    [property: Key(0)] long CommandId,
    [property: Key(1)] long Seq,
    [property: Key(2)] double Volts,
    [property: Key(3)] double BasedOnMs);

/// <summary>下り: 現行セッションの打ち切り。停止・異常時に送り、プラントは電圧を 0 にする。</summary>
[MessagePackObject]
public sealed record AbortCommand([property: Key(0)] long CommandId);

/// <summary>
/// 上り (プラント → コントローラ) の定周期帰還。生の物理量ではなくエンコーダのパルス数を送る点が実機と同じ。
/// </summary>
/// <param name="Seq">帰還連番</param>
/// <param name="CommandId">プラントが現在有効とみなしているセッション番号</param>
/// <param name="TimestampMs">計測時刻 (プラント時刻系) [ms]。送信処理時刻ではなく積分ステップの時刻。</param>
/// <param name="CartCounts">台車エンコーダのパルス数</param>
/// <param name="PendulumCounts">振子エンコーダのパルス数</param>
/// <param name="E2EMs">プラントが実測した「計測 → 指令適用」の遅延 [ms]</param>
/// <param name="DriveEnabled">ドライブが有効か</param>
/// <param name="Epoch">プラントのリセット世代。世代が変わったらコントローラは推定器を初期化する。</param>
[MessagePackObject]
public sealed record EncoderFeedback(
    [property: Key(0)] long Seq,
    [property: Key(1)] long CommandId,
    [property: Key(2)] double TimestampMs,
    [property: Key(3)] long CartCounts,
    [property: Key(4)] long PendulumCounts,
    [property: Key(5)] double E2EMs,
    [property: Key(6)] bool DriveEnabled,
    [property: Key(7)] int Epoch);

/// <summary>プラント側の保護が働いたことを知らせる即時イベント。定周期を待たずに送る (通信遅延は受ける)。</summary>
public enum MotionEventKind
{
    /// <summary>非常停止が押された</summary>
    EmergencyStop = 0,
    /// <summary>メカリミットに到達してドライブを遮断した</summary>
    LimitReached = 1,
    /// <summary>指令が途絶えたのでタイムアウトで電圧を 0 にした</summary>
    CommandTimeout = 2,
    /// <summary>ドライブ無効中に新しいセッションの指令が来たので拒否した</summary>
    DriveDisabled = 3,
}

/// <summary>上り: 保護動作の通知。</summary>
[MessagePackObject]
public sealed record MotionEvent(
    [property: Key(0)] MotionEventKind Kind,
    [property: Key(1)] long CommandId,
    [property: Key(2)] double TimestampMs,
    [property: Key(3)] long CartCounts,
    [property: Key(4)] long PendulumCounts);

/// <summary>コントローラの状態機械。</summary>
public enum ControlMode
{
    /// <summary>待機。指令を出さない。</summary>
    Idle = 0,
    /// <summary>エネルギー法で振り上げ中。</summary>
    SwingUp = 1,
    /// <summary>離散 LQR で倒立制御中。</summary>
    Balance = 2,
    /// <summary>異常停止。リセット操作でのみ復帰する。</summary>
    Fault = 3,
}

/// <summary>オペレータ操作。</summary>
public enum OperatorAction
{
    SwingUp = 0,
    Balance = 1,
    Stop = 2,
    ClearFault = 3,
}

/// <summary>通信路と帰還周期の設定。実機では純粋な観測値だが、実験のために注入できるようにしている。</summary>
[MessagePackObject]
public sealed record NetworkConfig(
    [property: Key(0)] int PeriodMs = 5,
    [property: Key(1)] double LatencyMs = 0.0,
    [property: Key(2)] double JitterMs = 0.0,
    [property: Key(3)] double SpikePercent = 0.0,
    [property: Key(4)] double EncoderNoiseCounts = 0.0)
{
    public const int MinPeriodMs = 1;
    public const int MaxPeriodMs = 100;

    /// <summary>範囲外の値で制御周期が壊れないよう、受信側で必ず通す。</summary>
    public NetworkConfig Sanitized() => new(
        Math.Clamp(PeriodMs, MinPeriodMs, MaxPeriodMs),
        Math.Clamp(LatencyMs, 0.0, 500.0),
        Math.Clamp(JitterMs, 0.0, 200.0),
        Math.Clamp(SpikePercent, 0.0, 100.0),
        Math.Clamp(EncoderNoiseCounts, 0.0, 50.0));
}

/// <summary>制御の任意オプション。</summary>
[MessagePackObject]
public sealed record ControlOptions(
    [property: Key(0)] bool UsePredictor = false,
    [property: Key(1)] double CartTargetMeters = 0.0,
    [property: Key(2)] double VelocityCutoffHz = 25.0,
    [property: Key(3)] double WatchdogMs = 300.0)
{
    public ControlOptions Sanitized() => new(
        UsePredictor,
        Math.Clamp(CartTargetMeters, -0.45, 0.45),
        Math.Clamp(VelocityCutoffHz, 1.0, 200.0),
        Math.Clamp(WatchdogMs, 50.0, 5000.0));
}

/// <summary>実験で変えるプラント側のパラメータ。コントローラの設計モデルにも同じ値を配る。</summary>
[MessagePackObject]
public sealed record PlantConfig([property: Key(0)] double PendulumLength = 0.6)
{
    public PlantConfig Sanitized() => new(Math.Clamp(PendulumLength, 0.1, 1.5));
}

/// <summary>コントローラの設計結果。帰還周期や振子長を変えるたびに配信する。</summary>
[MessagePackObject]
public sealed record DesignInfo(
    [property: Key(0)] double[] Gains,
    [property: Key(1)] double UnstablePole,
    [property: Key(2)] int DelayMarginMs,
    [property: Key(3)] int PeriodMs);

/// <summary>コントローラの現在状態。UI 表示用に 20Hz 程度で配信する。</summary>
[MessagePackObject]
public sealed record ControllerStatus(
    [property: Key(0)] ControlMode Mode,
    [property: Key(1)] string Reason,
    [property: Key(2)] long CommandId,
    [property: Key(3)] double Volts,
    [property: Key(4)] double TargetX,
    [property: Key(5)] double EstimatedX,
    [property: Key(6)] double EstimatedTheta,
    [property: Key(7)] double FeedbackIntervalMs,
    [property: Key(8)] double UplinkDelayMs,
    [property: Key(9)] double E2EDelayMs,
    [property: Key(10)] long FeedbackCount,
    [property: Key(11)] long StaleEventCount,
    [property: Key(12)] int DownlinkQueueLength);

public enum LogSeverity
{
    Info = 0,
    Warning = 1,
    Error = 2,
}

/// <summary>コントローラ側のイベントログ 1 行。</summary>
[MessagePackObject]
public sealed record LogEntry(
    [property: Key(0)] LogSeverity Level,
    [property: Key(1)] string Message,
    [property: Key(2)] ControlMode Mode,
    [property: Key(3)] long CommandId);

/// <summary>
/// NTP 方式のクロック同期。プラント時刻とコントローラ時刻の差を求めるために使う。
/// これがないと「上り遅延」は 2 つの端末の時計のズレを含んでしまう。
/// </summary>
[MessagePackObject]
public sealed record ClockSyncResult(
    [property: Key(0)] double ClientSendMs,
    [property: Key(1)] double ServerReceiveMs,
    [property: Key(2)] double ServerSendMs);

/// <summary>Hub のメソッド名。文字列の取り違えを防ぐために一箇所に集める。</summary>
public static class HubMethods
{
    // クライアント(プラント) → サーバ(コントローラ)
    public const string SendFeedback = nameof(SendFeedback);
    public const string SendMotionEvent = nameof(SendMotionEvent);
    public const string ConfigureNetwork = nameof(ConfigureNetwork);
    public const string ConfigureControl = nameof(ConfigureControl);
    public const string ConfigurePlant = nameof(ConfigurePlant);
    public const string Operate = nameof(Operate);
    public const string SyncClock = nameof(SyncClock);
    public const string ReportClockOffset = nameof(ReportClockOffset);

    // サーバ → クライアント
    public const string OnVoltage = nameof(OnVoltage);
    public const string OnAbort = nameof(OnAbort);
    public const string OnDesign = nameof(OnDesign);
    public const string OnStatus = nameof(OnStatus);
    public const string OnLog = nameof(OnLog);
}
