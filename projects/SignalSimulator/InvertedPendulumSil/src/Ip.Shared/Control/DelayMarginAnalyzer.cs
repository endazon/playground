using Ip.Shared.Numerics;

namespace Ip.Shared.Control;

/// <summary>
/// 閉ループが発散しないループ遅延の上限 (理論遅延余裕) を求める。
///
/// 解析的な位相余裕ではなく、実装と同じ構造 —— 周期 Ts のサンプル、差分+LPF の速度推定、
/// ZOH の電圧保持、一定遅延 —— を 1ms 刻みで数値シミュレーションし、
/// 発散するかどうかで二分探索する。実装と同じ構造で評価するため、
/// 「速度推定のフィルタ帯域を下げると余裕も減る」といった効果まで含めて評価できる。
/// </summary>
public static class DelayMarginAnalyzer
{
    private const double StepSeconds = 0.001;
    private const int SimulationSteps = 24_000; // 24 s
    private const int MaxDelayMs = 600;

    /// <param name="a">連続時間 A 行列</param>
    /// <param name="b">連続時間 B ベクトル</param>
    /// <param name="k">状態フィードバックゲイン</param>
    /// <param name="tsSeconds">帰還周期 [s]</param>
    /// <param name="cutoffHz">速度推定 LPF のカットオフ [Hz]</param>
    /// <returns>倒立を維持できる最大のループ遅延 [ms]。0 なら遅延ゼロでも安定化できない。</returns>
    public static int Compute(double[,] a, double[] b, double[] k, double tsSeconds, double cutoffHz)
    {
        var (ad, bd) = Discretization.ZeroOrderHold(a, b, StepSeconds);
        int subSteps = Math.Max(1, (int)Math.Round(tsSeconds / StepSeconds));
        double lpf = VelocityEstimator.LowPassGain(cutoffHz, subSteps * StepSeconds);

        bool Unstable(int delayMs)
        {
            // 遅延キュー: (適用ステップ, 電圧) を FIFO で保持する
            var queue = new Queue<(int At, double U)>();
            double u = 0.0;
            var x = new double[] { 0.0, 0.01, 0.0, 0.0 };
            double prevX = 0.0, prevTh = 0.01, vx = 0.0, vth = 0.0;
            double firstHalfPeak = 0.0, secondHalfPeak = 0.0;
            double dt = subSteps * StepSeconds;

            for (int step = 0; step < SimulationSteps; step++)
            {
                if (step % subSteps == 0)
                {
                    vx += lpf * ((x[0] - prevX) / dt - vx);
                    vth += lpf * ((x[1] - prevTh) / dt - vth);
                    prevX = x[0];
                    prevTh = x[1];
                    queue.Enqueue((step + delayMs, -(k[0] * x[0] + k[1] * x[1] + k[2] * vx + k[3] * vth)));
                }

                while (queue.Count > 0 && queue.Peek().At <= step) u = queue.Dequeue().U;

                var nx = Matrix.Multiply(ad, x);
                for (int i = 0; i < 4; i++) nx[i] += bd[i] * u;
                x = nx;

                double magnitude = Math.Abs(x[1]);
                if (!(magnitude < 1e3)) return true;
                if (step > SimulationSteps / 3 && step <= 2 * SimulationSteps / 3)
                    firstHalfPeak = Math.Max(firstHalfPeak, magnitude);
                else if (step > 2 * SimulationSteps / 3)
                    secondHalfPeak = Math.Max(secondHalfPeak, magnitude);
            }

            // 発散はしていないが減衰もしていない (振幅が保たれている) 場合も不安定とみなす
            return secondHalfPeak > firstHalfPeak * 0.98 && secondHalfPeak > 1e-4;
        }

        if (Unstable(0)) return 0;
        if (!Unstable(MaxDelayMs)) return MaxDelayMs;

        int lo = 0, hi = MaxDelayMs;
        while (hi - lo > 1)
        {
            int mid = (lo + hi) / 2;
            if (Unstable(mid)) hi = mid; else lo = mid;
        }
        return lo;
    }
}
