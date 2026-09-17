namespace Ip.Shared.Model;

/// <summary>
/// 台車型倒立振子の物理パラメータ。プラント(仮想ハードウェア)とコントローラの双方が同じ定義を参照する。
/// コントローラ側が持つのは「設計モデル」であり、プラント側の真値とずらしてモデル誤差の実験もできる。
/// </summary>
public sealed record PendulumParameters
{
    /// <summary>重力加速度 [m/s^2]</summary>
    public const double Gravity = 9.81;

    // --- 台車・駆動系 ---
    /// <summary>台車質量 [kg]</summary>
    public double CartMass { get; init; } = 0.6;
    /// <summary>モータロータ慣性 [kg m^2]</summary>
    public double RotorInertia { get; init; } = 2e-5;
    /// <summary>プーリ半径 [m] (回転→並進の変換比)</summary>
    public double PulleyRadius { get; init; } = 0.015;
    /// <summary>トルク定数 [Nm/A]</summary>
    public double TorqueConstant { get; init; } = 0.05;
    /// <summary>逆起電力定数 [Vs/rad]</summary>
    public double BackEmfConstant { get; init; } = 0.05;
    /// <summary>巻線抵抗 [ohm]</summary>
    public double MotorResistance { get; init; } = 2.0;
    /// <summary>電源電圧 = 指令電圧の飽和値 [V]</summary>
    public double MaxVoltage { get; init; } = 24.0;
    /// <summary>台車の粘性摩擦 [Ns/m]</summary>
    public double CartViscousFriction { get; init; } = 1.0;
    /// <summary>台車のクーロン摩擦 [N]</summary>
    public double CartCoulombFriction { get; init; } = 0.4;

    // --- 振子 ---
    /// <summary>振子質量 [kg]</summary>
    public double PendulumMass { get; init; } = 0.15;
    /// <summary>振子全長 [m] (重心は L/2)</summary>
    public double PendulumLength { get; init; } = 0.6;
    /// <summary>振子軸の粘性摩擦 [Nms/rad]</summary>
    public double PivotViscousFriction { get; init; } = 8e-4;

    // --- 機構・センサ ---
    /// <summary>レール片側ストローク [m]</summary>
    public double RailStroke { get; init; } = 0.5;
    /// <summary>台車エンコーダ分解能 [count/rev] (4逓倍後)</summary>
    public int CartEncoderCountsPerRev { get; init; } = 2000;
    /// <summary>振子エンコーダ分解能 [count/rev] (4逓倍後)</summary>
    public int PendulumEncoderCountsPerRev { get; init; } = 4096;
    /// <summary>メカストッパ剛性 [N/m]</summary>
    public double StopStiffness { get; init; } = 2e4;
    /// <summary>メカストッパ減衰 [Ns/m]</summary>
    public double StopDamping { get; init; } = 150.0;

    /// <summary>派生量をまとめて計算する。パラメータ変更のたびに 1 回だけ呼ぶ。</summary>
    public DerivedParameters Derive() => DerivedParameters.From(this);
}

/// <summary>
/// <see cref="PendulumParameters"/> から一度だけ計算しておく派生量。積分ループの内側で毎回割り算をしないための構造体。
/// </summary>
public readonly record struct DerivedParameters
{
    /// <summary>軸から重心までの距離 [m]</summary>
    public required double HalfLength { get; init; }
    /// <summary>軸まわりの慣性モーメント [kg m^2] (棒の重心慣性 + 平行軸)</summary>
    public required double PivotInertia { get; init; }
    /// <summary>ロータ慣性を並進換算して加えた等価台車質量 [kg]</summary>
    public required double EquivalentCartMass { get; init; }
    /// <summary>電圧→推力ゲイン [N/V]</summary>
    public required double ForceGain { get; init; }
    /// <summary>逆起電力による速度比例抗力 [Ns/m]</summary>
    public required double BackEmfDamping { get; init; }
    /// <summary>台車エンコーダ 1 カウントあたりの移動量 [m/count]</summary>
    public required double MetersPerCount { get; init; }

    public static DerivedParameters From(PendulumParameters p)
    {
        double halfLength = p.PendulumLength / 2.0;
        double centerInertia = p.PendulumMass * p.PendulumLength * p.PendulumLength / 12.0;
        return new DerivedParameters
        {
            HalfLength = halfLength,
            PivotInertia = centerInertia + p.PendulumMass * halfLength * halfLength,
            EquivalentCartMass = p.CartMass + p.RotorInertia / (p.PulleyRadius * p.PulleyRadius),
            ForceGain = p.TorqueConstant / (p.MotorResistance * p.PulleyRadius),
            BackEmfDamping = p.TorqueConstant * p.BackEmfConstant
                             / (p.MotorResistance * p.PulleyRadius * p.PulleyRadius),
            MetersPerCount = 2.0 * Math.PI * p.PulleyRadius / p.CartEncoderCountsPerRev,
        };
    }
}
