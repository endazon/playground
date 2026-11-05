using Ip.Shared.Control;
using Ip.Shared.Model;
using Ip.Shared.Protocol;

namespace Ip.Shared.Tests;

/// <summary>
/// 受信値の矯正。Math.Clamp は Infinity を境界へ落とすが NaN は素通しするため、
/// 1 つ混ざると「比較がすべて false になる」形の静かな故障を生む。
/// </summary>
public sealed class SanitizationTests
{
    public static TheoryData<double> NonFiniteValues => new() { double.NaN, double.PositiveInfinity, double.NegativeInfinity };

    [Theory]
    [MemberData(nameof(NonFiniteValues))]
    public void NetworkConfig_Sanitized_IsAlwaysFinite(double bad)
    {
        var sanitized = new NetworkConfig(5, bad, bad, bad, bad).Sanitized();

        Assert.True(double.IsFinite(sanitized.LatencyMs));
        Assert.True(double.IsFinite(sanitized.JitterMs));
        Assert.True(double.IsFinite(sanitized.SpikePercent));
        Assert.True(double.IsFinite(sanitized.EncoderNoiseCounts));
    }

    [Theory]
    [MemberData(nameof(NonFiniteValues))]
    public void ControlOptions_Sanitized_IsAlwaysFinite(double bad)
    {
        var sanitized = new ControlOptions(false, bad, bad, bad).Sanitized();

        Assert.True(double.IsFinite(sanitized.CartTargetMeters));
        Assert.True(double.IsFinite(sanitized.VelocityCutoffHz));
        Assert.True(double.IsFinite(sanitized.WatchdogMs));
        Assert.InRange(sanitized.WatchdogMs, 50.0, 5000.0);
    }

    [Theory]
    [MemberData(nameof(NonFiniteValues))]
    public void PlantConfig_Sanitized_IsAlwaysFinite(double bad)
    {
        var sanitized = new PlantConfig(bad).Sanitized();

        Assert.True(double.IsFinite(sanitized.PendulumLength));
        Assert.InRange(sanitized.PendulumLength, 0.1, 1.5);
    }

    [Fact]
    public void CartTargetLimit_LeavesRoomBeforeTheSoftLimit()
    {
        // 目標位置の上限をソフトリミットと同値にすると、正規の最大値を入れただけで
        // 整定時のオーバーシュートで必ず FAULT に落ちる。
        Assert.True(CartLimits.MaxTargetM < CartLimits.SoftLimitM,
            $"目標上限 {CartLimits.MaxTargetM} がソフトリミット {CartLimits.SoftLimitM} と同値以上");
        Assert.Equal(CartLimits.SoftLimitM - CartLimits.TargetOvershootMarginM, CartLimits.MaxTargetM, 12);

        var clamped = new ControlOptions(CartTargetMeters: 10.0).Sanitized();
        Assert.Equal(CartLimits.MaxTargetM, clamped.CartTargetMeters, 12);
    }

    [Fact]
    public void SoftLimit_MatchesTheControllerConstants()
    {
        // 2 箇所にハードコードして片方だけ直す事故を防ぐ
        var parameters = new PendulumParameters();
        Assert.Equal(CartLimits.RailStrokeM, parameters.RailStroke, 12);
        Assert.Equal(CartLimits.SoftLimitMarginM, ControllerCore.SoftLimitMarginM, 12);
    }
}
