using Ip.Shared.Model;

namespace Ip.Shared.Control;

/// <summary>
/// 倒立点 (θ=0) まわりで線形化した連続時間モデル。入力は電圧 [V]、状態は [x, θ, ẋ, θ̇]。
/// </summary>
public sealed class LinearizedModel
{
    private LinearizedModel(double[,] a, double[] b, double unstablePole)
    {
        A = a;
        B = b;
        UnstablePole = unstablePole;
    }

    public double[,] A { get; }
    public double[] B { get; }

    /// <summary>倒立点の不安定極 λ [rad/s]。倒れ始めの時定数はおおよそ 1/λ。</summary>
    public double UnstablePole { get; }

    public static LinearizedModel Create(PendulumParameters p)
    {
        var d = p.Derive();
        double a11 = d.EquivalentCartMass + p.PendulumMass;
        double a12 = p.PendulumMass * d.HalfLength;
        double det = a11 * d.PivotInertia - a12 * a12;
        double mgl = p.PendulumMass * PendulumParameters.Gravity * d.HalfLength;
        double bx = d.BackEmfDamping + p.CartViscousFriction;

        var a = new double[4, 4];
        a[0, 2] = 1.0;
        a[1, 3] = 1.0;
        a[2, 1] = -a12 * mgl / det;
        a[2, 2] = -d.PivotInertia * bx / det;
        a[2, 3] = a12 * p.PivotViscousFriction / det;
        a[3, 1] = a11 * mgl / det;
        a[3, 2] = a12 * bx / det;
        a[3, 3] = -a11 * p.PivotViscousFriction / det;

        var b = new double[4];
        b[2] = d.PivotInertia * d.ForceGain / det;
        b[3] = -a12 * d.ForceGain / det;

        return new LinearizedModel(a, b, Math.Sqrt(a11 * mgl / det));
    }
}
