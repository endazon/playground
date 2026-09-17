using Ip.Shared.Control;
using Ip.Shared.Model;

namespace Ip.Shared.Tests;

public sealed class PhysicsTests
{
    private static readonly PendulumParameters Frictionless = new()
    {
        CartViscousFriction = 0.0,
        CartCoulombFriction = 0.0,
        PivotViscousFriction = 0.0,
        BackEmfConstant = 0.0,
    };

    [Fact]
    public void WrapAngle_NormalizesToPrincipalRange()
    {
        Assert.Equal(0.0, CartPoleDynamics.WrapAngle(2.0 * Math.PI), 12);
        Assert.Equal(-0.1, CartPoleDynamics.WrapAngle(2.0 * Math.PI - 0.1), 12);
        Assert.Equal(0.1, CartPoleDynamics.WrapAngle(-2.0 * Math.PI + 0.1), 12);
        Assert.InRange(CartPoleDynamics.WrapAngle(103.7), -Math.PI, Math.PI);
    }

    [Fact]
    public void HangingState_IsAnEquilibrium()
    {
        var p = new PendulumParameters();
        var d = p.Derive();

        var next = CartPoleDynamics.Rk4Step(p, d, PlantState.Hanging, 0.0, Disturbance.None, 0.001);

        Assert.Equal(PlantState.Hanging.X, next.X, 12);
        Assert.Equal(PlantState.Hanging.Theta, next.Theta, 12);
        Assert.Equal(0.0, next.XDot, 12);
        Assert.Equal(0.0, next.ThetaDot, 12);
    }

    [Fact]
    public void UprightState_IsUnstable_AndFallsTowardTheTiltDirection()
    {
        var p = new PendulumParameters();
        var d = p.Derive();
        var s = new PlantState(0.0, 0.01, 0.0, 0.0);

        for (int i = 0; i < 1000; i++)
            s = CartPoleDynamics.Rk4Step(p, d, s, 0.0, Disturbance.None, 0.001);

        Assert.True(s.Theta > 0.01, "+θ 側へ傾けたのに倒れていない");
    }

    [Fact]
    public void FreeSwing_ConservesEnergy_WhenFrictionIsRemoved()
    {
        var p = Frictionless;
        var d = p.Derive();
        var s = new PlantState(0.0, 2.0, 0.0, 0.0); // 水平より下から放す

        double initial = TotalEnergy(p, d, s);
        for (int i = 0; i < 5000; i++)
            s = CartPoleDynamics.Rk4Step(p, d, s, 0.0, Disturbance.None, 0.001);
        double final = TotalEnergy(p, d, s);

        Assert.Equal(initial, final, 4); // RK4 の 1ms 刻みで 5 秒積分しても 1e-4 J 以内
    }

    [Fact]
    public void MechanicalStop_KeepsCartInsideTheRail()
    {
        var p = new PendulumParameters();
        var d = p.Derive();
        var s = new PlantState(0.0, Math.PI, 0.0, 0.0);

        // 全開電圧を 3 秒間かけ続けてもストッパを突き破らない
        for (int i = 0; i < 3000; i++)
            s = CartPoleDynamics.Rk4Step(p, d, s, p.MaxVoltage, Disturbance.None, 0.001);

        Assert.True(s.X < p.RailStroke + 0.02, $"ストッパを突き抜けた: x={s.X:0.000}");
    }

    [Fact]
    public void PendulumEnergy_IsZeroAtUprightRest_AndNegativeWhenHanging()
    {
        var p = new PendulumParameters();
        var d = p.Derive();

        Assert.Equal(0.0, CartPoleDynamics.PendulumEnergy(p, d, PlantState.NearUpright(0.0)), 12);
        Assert.True(CartPoleDynamics.PendulumEnergy(p, d, PlantState.Hanging) < 0.0);
    }

    [Theory]
    [InlineData(0.3, 7.5)]
    [InlineData(0.6, 5.3)]
    [InlineData(1.0, 4.1)]
    public void UnstablePole_MatchesTheDesignNote(double length, double expectedPole)
    {
        var model = LinearizedModel.Create(new PendulumParameters { PendulumLength = length });

        Assert.Equal(expectedPole, model.UnstablePole, 1);
    }

    [Fact]
    public void LinearizedModel_MatchesNonlinearDerivativeNearUpright()
    {
        // 線形モデルはクーロン摩擦 (原点で微分不可能) を含まないので、比較もそれを外した条件で行う。
        // ここで無視した摩擦は、そのままモデル誤差としてコントローラに残る。
        var p = new PendulumParameters { CartCoulombFriction = 0.0 };
        var d = p.Derive();
        var model = LinearizedModel.Create(p);
        var s = new PlantState(0.01, 0.005, 0.02, 0.01);
        const double volts = 1.3;

        var exact = CartPoleDynamics.Derivative(p, d, s, volts, Disturbance.None);
        var x = s.ToArray();
        var linear = new double[4];
        for (int i = 0; i < 4; i++)
        {
            double sum = model.B[i] * volts;
            for (int j = 0; j < 4; j++) sum += model.A[i, j] * x[j];
            linear[i] = sum;
        }

        Assert.Equal(exact.XDot, linear[2], 2);
        Assert.Equal(exact.ThetaDot, linear[3], 2);
    }

    private static double TotalEnergy(PendulumParameters p, in DerivedParameters d, in PlantState s)
    {
        // 台車の運動エネルギー + 振子の運動エネルギー + 位置エネルギー
        double cart = 0.5 * (d.EquivalentCartMass + p.PendulumMass) * s.XDot * s.XDot;
        double coupling = p.PendulumMass * d.HalfLength * Math.Cos(s.Theta) * s.XDot * s.ThetaDot;
        double pendulum = 0.5 * d.PivotInertia * s.ThetaDot * s.ThetaDot;
        double potential = p.PendulumMass * PendulumParameters.Gravity * d.HalfLength * Math.Cos(s.Theta);
        return cart + coupling + pendulum + potential;
    }
}
