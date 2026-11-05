using Ip.Shared.Numerics;

namespace Ip.Shared.Tests;

public sealed class NumericsTests
{
    [Fact]
    public void Exp_OfZeroMatrix_IsIdentity()
    {
        var e = Matrix.Exp(new double[3, 3]);
        for (int i = 0; i < 3; i++)
            for (int j = 0; j < 3; j++)
                Assert.Equal(i == j ? 1.0 : 0.0, e[i, j], 12);
    }

    [Fact]
    public void Exp_OfDiagonalMatrix_MatchesScalarExponential()
    {
        var m = new double[2, 2];
        m[0, 0] = 1.5;
        m[1, 1] = -3.0;

        var e = Matrix.Exp(m);

        Assert.Equal(Math.Exp(1.5), e[0, 0], 10);
        Assert.Equal(Math.Exp(-3.0), e[1, 1], 10);
        Assert.Equal(0.0, e[0, 1], 12);
    }

    [Fact]
    public void Exp_OfRotationGenerator_MatchesRotationMatrix()
    {
        // [[0,-θ],[θ,0]] の指数は回転行列になる
        const double theta = 0.7;
        var m = new double[2, 2];
        m[0, 1] = -theta;
        m[1, 0] = theta;

        var e = Matrix.Exp(m);

        Assert.Equal(Math.Cos(theta), e[0, 0], 10);
        Assert.Equal(-Math.Sin(theta), e[0, 1], 10);
        Assert.Equal(Math.Sin(theta), e[1, 0], 10);
        Assert.Equal(Math.Cos(theta), e[1, 1], 10);
    }

    [Fact]
    public void ZeroOrderHold_OfDoubleIntegrator_MatchesAnalyticSolution()
    {
        // ẋ = [[0,1],[0,0]] x + [0,1] u  →  Ad = [[1,Ts],[0,1]], Bd = [Ts^2/2, Ts]
        const double ts = 0.02;
        var a = new double[2, 2];
        a[0, 1] = 1.0;
        var b = new double[] { 0.0, 1.0 };

        var (ad, bd) = Discretization.ZeroOrderHold(a, b, ts);

        Assert.Equal(1.0, ad[0, 0], 12);
        Assert.Equal(ts, ad[0, 1], 12);
        Assert.Equal(0.0, ad[1, 0], 12);
        Assert.Equal(1.0, ad[1, 1], 12);
        Assert.Equal(ts * ts / 2.0, bd[0], 12);
        Assert.Equal(ts, bd[1], 12);
    }

    [Fact]
    public void DiscreteLqr_StabilizesDoubleIntegrator()
    {
        const double ts = 0.01;
        var a = new double[2, 2];
        a[0, 1] = 1.0;
        var b = new double[] { 0.0, 1.0 };
        var (ad, bd) = Discretization.ZeroOrderHold(a, b, ts);

        var k = DiscreteLqr.Solve(ad, bd, [1.0, 1.0], 1.0);

        // 閉ループを回して収束することを確認する (極計算を持ち込まずに安定性を見る)
        double[] x = [1.0, 0.0];
        for (int i = 0; i < 5000; i++)
        {
            double u = -(k[0] * x[0] + k[1] * x[1]);
            double[] next = [ad[0, 0] * x[0] + ad[0, 1] * x[1] + bd[0] * u,
                             ad[1, 0] * x[0] + ad[1, 1] * x[1] + bd[1] * u];
            x = next;
        }

        Assert.True(Math.Abs(x[0]) < 1e-6, $"x0 が収束していない: {x[0]}");
        Assert.True(Math.Abs(x[1]) < 1e-6, $"x1 が収束していない: {x[1]}");
    }

    [Fact]
    public void DiscreteLqr_HeavierInputWeight_ProducesSmallerGain()
    {
        const double ts = 0.01;
        var a = new double[2, 2];
        a[0, 1] = 1.0;
        var b = new double[] { 0.0, 1.0 };
        var (ad, bd) = Discretization.ZeroOrderHold(a, b, ts);

        var cheap = DiscreteLqr.Solve(ad, bd, [1.0, 1.0], 0.01);
        var expensive = DiscreteLqr.Solve(ad, bd, [1.0, 1.0], 100.0);

        Assert.True(expensive[0] < cheap[0]);
    }
}
