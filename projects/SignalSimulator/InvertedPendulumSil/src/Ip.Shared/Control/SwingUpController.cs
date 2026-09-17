using Ip.Shared.Model;

namespace Ip.Shared.Control;

/// <summary>エネルギー法スイングアップの調整ゲイン。</summary>
public sealed record SwingUpGains
{
    /// <summary>エネルギー誤差ゲイン</summary>
    public double EnergyGain { get; init; } = 80.0;
    /// <summary>加速度指令の振幅 [m/s^2]</summary>
    public double AccelerationAmplitude { get; init; } = 3.0;
    /// <summary>台車位置の引き戻しゲイン</summary>
    public double PositionGain { get; init; } = 5.0;
    /// <summary>台車速度の減衰ゲイン</summary>
    public double VelocityGain { get; init; } = 3.0;
    /// <summary>目標エネルギーのオフセット [J]。わずかに過剰にして倒立領域へ入りやすくする。</summary>
    public double EnergyOffset { get; init; } = 0.05;
    /// <summary>加速度指令の飽和 [m/s^2]</summary>
    public double AccelerationLimit { get; init; } = 5.0;
    /// <summary>電圧の飽和 [V]。倒立制御より低く抑えて突入速度を落とす。</summary>
    public double VoltageLimit { get; init; } = 18.0;
}

/// <summary>
/// エネルギー法によるスイングアップ。
///
/// 振子のエネルギーの時間微分は dE/dt = -m·l·cosθ·θ̇·ẍ なので、
/// ẍ ∝ (E - E*)·sgn(cosθ·θ̇) と置けば E は単調に目標エネルギーへ近づく。
/// 台車の加速度指令を、逆モデルで電圧に直してから出力する。
/// </summary>
public static class SwingUpController
{
    public static double Voltage(
        PendulumParameters p, in DerivedParameters d, in PlantState s, SwingUpGains g)
    {
        double energy = CartPoleDynamics.PendulumEnergy(p, d, s);
        double sign = Math.Sign(s.ThetaDot * Math.Cos(s.Theta));
        if (sign == 0.0) sign = 1.0;

        double accel = g.AccelerationAmplitude
                       * CartPoleDynamics.Clamp(g.EnergyGain * (energy - g.EnergyOffset), -1.0, 1.0) * sign
                       - g.PositionGain * s.X
                       - g.VelocityGain * s.XDot;
        accel = CartPoleDynamics.Clamp(accel, -g.AccelerationLimit, g.AccelerationLimit);

        // 加速度 → 推力 → 電圧 (逆起電力と摩擦を打ち消す前向き補償を含む)
        double volts = ((d.EquivalentCartMass + p.PendulumMass) * accel
                        + (d.BackEmfDamping + p.CartViscousFriction) * s.XDot
                        + p.CartCoulombFriction * Math.Tanh(s.XDot / 0.01)) / d.ForceGain;

        return CartPoleDynamics.Clamp(volts, -g.VoltageLimit, g.VoltageLimit);
    }
}
