namespace Ip.Shared.Model;

/// <summary>
/// 台車型倒立振子の非線形運動方程式と 4 次 Runge-Kutta 積分。
/// 入力は電圧 [V]。DC モータの逆起電力・台車の粘性/クーロン摩擦・メカストッパを含む。
/// </summary>
public static class CartPoleDynamics
{
    /// <summary>状態微分 ds/dt を返す。</summary>
    public static PlantState Derivative(
        PendulumParameters p, in DerivedParameters d, in PlantState s, double volts, in Disturbance dist)
    {
        double c = Math.Cos(s.Theta);
        double sn = Math.Sin(s.Theta);

        // 電圧→推力。逆起電力ぶんは速度に比例する抗力として現れる。
        double force = d.ForceGain * volts
                       - (d.BackEmfDamping + p.CartViscousFriction) * s.XDot
                       - p.CartCoulombFriction * Math.Tanh(s.XDot / 0.01)
                       + dist.Force;

        // メカストッパ: ばね+ダンパ。押し戻す向きの力しか出さない(引き込まない)。
        double penetration = s.X > p.RailStroke ? s.X - p.RailStroke
            : s.X < -p.RailStroke ? s.X + p.RailStroke
            : 0.0;
        if (penetration != 0.0)
        {
            double stopForce = -p.StopStiffness * penetration - p.StopDamping * s.XDot;
            if (stopForce * penetration > 0.0) stopForce = 0.0;
            force += stopForce;
        }

        double a11 = d.EquivalentCartMass + p.PendulumMass;
        double a12 = p.PendulumMass * d.HalfLength * c;
        double det = a11 * d.PivotInertia - a12 * a12;

        double b1 = force + p.PendulumMass * d.HalfLength * sn * s.ThetaDot * s.ThetaDot;
        double b2 = p.PendulumMass * PendulumParameters.Gravity * d.HalfLength * sn
                    - p.PivotViscousFriction * s.ThetaDot
                    + dist.Torque;

        return new PlantState(
            s.XDot,
            s.ThetaDot,
            (b1 * d.PivotInertia - a12 * b2) / det,
            (a11 * b2 - a12 * b1) / det);
    }

    /// <summary>刻み h [s] の RK4 で 1 ステップ進める。</summary>
    public static PlantState Rk4Step(
        PendulumParameters p, in DerivedParameters d, in PlantState s, double volts, in Disturbance dist, double h)
    {
        PlantState k1 = Derivative(p, d, s, volts, dist);
        PlantState k2 = Derivative(p, d, s + k1 * (h / 2.0), volts, dist);
        PlantState k3 = Derivative(p, d, s + k2 * (h / 2.0), volts, dist);
        PlantState k4 = Derivative(p, d, s + k3 * h, volts, dist);
        return s + (k1 + (k2 + k3) * 2.0 + k4) * (h / 6.0);
    }

    /// <summary>倒立静止を 0 とした振子の力学エネルギー [J]。スイングアップの評価量。</summary>
    public static double PendulumEnergy(PendulumParameters p, in DerivedParameters d, in PlantState s)
        => 0.5 * d.PivotInertia * s.ThetaDot * s.ThetaDot
           + p.PendulumMass * PendulumParameters.Gravity * d.HalfLength * (Math.Cos(s.Theta) - 1.0);

    /// <summary>角度を [-π, π) に正規化する (π は -π に写る)。</summary>
    public static double WrapAngle(double a)
    {
        a = (a + Math.PI) % (2.0 * Math.PI);
        if (a < 0.0) a += 2.0 * Math.PI;
        return a - Math.PI;
    }

    public static double Clamp(double v, double lo, double hi) => v < lo ? lo : v > hi ? hi : v;
}
