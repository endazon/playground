namespace Ip.Shared.Control;

/// <summary>
/// 位置差分 + 1 次ローパスによる速度推定。
/// エンコーダの量子化ノイズを差分で増幅してしまうため、帯域 <paramref name="CutoffHz"/> で落とす。
/// 可変周期の帰還に対応するため、ゲインはサンプルごとに実測 dt から計算する。
/// </summary>
public sealed class VelocityEstimator(double cutoffHz)
{
    public double CutoffHz { get; set; } = cutoffHz;

    public double Value { get; private set; }
    public double LastInput { get; private set; }
    public bool Initialized { get; private set; }

    /// <summary>1 次ローパスの離散ゲイン 1 - exp(-2π fc dt)。</summary>
    public static double LowPassGain(double cutoffHz, double dtSeconds)
        => 1.0 - Math.Exp(-2.0 * Math.PI * cutoffHz * dtSeconds);

    public void Reset(double input)
    {
        LastInput = input;
        Value = 0.0;
        Initialized = true;
    }

    /// <summary>新しい計測値で更新して推定速度を返す。dt&lt;=0 のサンプルは無視する。</summary>
    public double Update(double input, double dtSeconds)
    {
        if (!Initialized)
        {
            Reset(input);
            return Value;
        }
        if (dtSeconds > 0.0)
        {
            double a = LowPassGain(CutoffHz, dtSeconds);
            Value += a * ((input - LastInput) / dtSeconds - Value);
        }
        LastInput = input;
        return Value;
    }
}
