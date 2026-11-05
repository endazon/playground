using Ip.Shared.Control;
using Ip.Shared.Model;
using Xunit.Abstractions;

namespace Ip.Shared.Tests;

/// <summary>
/// 設計メモに載せた遅延余裕の表が、実装から再現できることを確かめる。
/// この表が「純 SIL がどこまで成立するか」の根拠なので、実装を変えたら必ずここで検算する。
/// </summary>
public sealed class DelayMarginTests(ITestOutputHelper output)
{
    private const double CutoffHz = 25.0;
    private static readonly int[] Periods = [1, 5, 20, 50, 100];

    /// <summary>設計メモに貼る表を出力する。README / 画面の数字はここから取る。</summary>
    [Fact]
    public void PrintTable()
    {
        output.WriteLine("振子長 | 不安定極 | " + string.Join(" | ", Periods.Select(p => $"{p}ms")));
        foreach (double length in new[] { 0.3, 0.6, 1.0 })
        {
            var p = new PendulumParameters { PendulumLength = length };
            var margins = Periods.Select(period => ControllerCore.CreateDesign(p, period, CutoffHz).Info.DelayMarginMs);
            var design = ControllerCore.CreateDesign(p, 5, CutoffHz);
            var values = margins.ToArray();
            output.WriteLine($"{length:0.0} m | {design.Info.UnstablePole:0.0} rad/s | {string.Join(" | ", values)}");
            Assert.All(values, m => Assert.True(m > 0, "遅延ゼロでも安定化できない設計になっている"));
        }
    }

    [Theory]
    [InlineData(0.3, 1, 58)]
    [InlineData(0.3, 5, 57)]
    [InlineData(0.3, 20, 52)]
    [InlineData(0.3, 50, 41)]
    [InlineData(0.3, 100, 35)]
    [InlineData(0.6, 1, 86)]
    [InlineData(0.6, 5, 85)]
    [InlineData(0.6, 20, 80)]
    [InlineData(0.6, 50, 68)]
    [InlineData(0.6, 100, 59)]
    [InlineData(1.0, 1, 110)]
    [InlineData(1.0, 5, 109)]
    [InlineData(1.0, 20, 105)]
    [InlineData(1.0, 50, 95)]
    [InlineData(1.0, 100, 86)]
    public void DelayMargin_MatchesTheDesignNote(double length, int periodMs, int expectedMs)
    {
        var design = ControllerCore.CreateDesign(
            new PendulumParameters { PendulumLength = length }, periodMs, CutoffHz);

        Assert.InRange(design.Info.DelayMarginMs, expectedMs - 2, expectedMs + 2);
    }

    [Fact]
    public void DelayMargin_ShrinksAsThePendulumGetsShorter()
    {
        int longRod = Margin(1.0);
        int mediumRod = Margin(0.6);
        int shortRod = Margin(0.3);

        Assert.True(longRod > mediumRod);
        Assert.True(mediumRod > shortRod);

        static int Margin(double length) => ControllerCore
            .CreateDesign(new PendulumParameters { PendulumLength = length }, 5, CutoffHz).Info.DelayMarginMs;
    }

    [Fact]
    public void DelayMargin_ShrinksAsTheFeedbackPeriodGrows()
    {
        var p = new PendulumParameters();
        int fast = ControllerCore.CreateDesign(p, 1, CutoffHz).Info.DelayMarginMs;
        int slow = ControllerCore.CreateDesign(p, 100, CutoffHz).Info.DelayMarginMs;

        Assert.True(fast > slow, $"周期を伸ばしても余裕が減っていない: {fast} → {slow}");
    }
}
