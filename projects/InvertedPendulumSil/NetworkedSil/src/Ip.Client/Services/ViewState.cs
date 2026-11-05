namespace Ip.Client.Services;

/// <summary>
/// 描画モジュールへ 50Hz で渡す状態。JS 側では camelCase になる。
/// 描画に必要な最小限だけを入れる (JS へ渡す量が増えるほど、相互運用のコストがフレームごとに乗るため)。
/// </summary>
/// <param name="SimSeconds">プラントのシミュレーション時刻 [s]</param>
/// <param name="X">台車位置 [m]</param>
/// <param name="Theta">振子角 [rad]</param>
/// <param name="Volts">印加電圧 [V]</param>
/// <param name="DriveEnabled">ドライブ有効か</param>
/// <param name="LatencyMs">プラントが実測した E2E 遅延 [ms]</param>
/// <param name="HasEstimate">バック側の推定値が有効か</param>
/// <param name="EstX">バックが見ている台車位置 [m]</param>
/// <param name="EstTheta">バックが見ている振子角 [rad]</param>
/// <param name="TargetX">台車の目標位置 [m]</param>
/// <param name="Running">運転中 (SWINGUP / BALANCE) か</param>
/// <param name="PendulumLength">振子長 [m]</param>
/// <param name="MarginMs">理論遅延余裕 [ms]</param>
public readonly record struct ViewState(
    double SimSeconds,
    double X,
    double Theta,
    double Volts,
    bool DriveEnabled,
    double LatencyMs,
    bool HasEstimate,
    double EstX,
    double EstTheta,
    double TargetX,
    bool Running,
    double PendulumLength,
    int MarginMs);
