namespace Ip.Shared.Numerics;

/// <summary>
/// 単入力系の離散時間 LQR。リカッチ差分方程式を反復して収束させる。
/// 更新式は Joseph 形式 + 明示的な対称化で、数値誤差による非対称化から来る発散を防いでいる。
/// </summary>
public static class DiscreteLqr
{
    public const int DefaultMaxIterations = 40_000;
    private const double RelativeTolerance = 1e-11;

    /// <param name="ad">離散系の A 行列</param>
    /// <param name="bd">離散系の B ベクトル (単入力)</param>
    /// <param name="qDiagonal">状態重みの対角成分</param>
    /// <param name="r">入力重み (スカラ)</param>
    /// <returns>状態フィードバックゲイン K。入力は u = -K x。</returns>
    public static double[] Solve(double[,] ad, double[] bd, double[] qDiagonal, double r,
        int maxIterations = DefaultMaxIterations)
    {
        int n = ad.GetLength(0);
        if (bd.Length != n || qDiagonal.Length != n) throw new ArgumentException("次元が一致しません。");
        if (r <= 0.0) throw new ArgumentOutOfRangeException(nameof(r), "入力重みは正である必要があります。");

        var p = new double[n, n];
        for (int i = 0; i < n; i++) p[i, i] = qDiagonal[i];
        var k = new double[n];
        var acl = new double[n, n];

        for (int iteration = 0; iteration < maxIterations; iteration++)
        {
            // K = (R + B'PB)^-1 B'PA
            var pb = Matrix.Multiply(p, bd);
            double bpb = 0.0;
            for (int i = 0; i < n; i++) bpb += bd[i] * pb[i];
            for (int j = 0; j < n; j++)
            {
                double s = 0.0;
                for (int i = 0; i < n; i++) s += pb[i] * ad[i, j];
                k[j] = s / (r + bpb);
            }

            // Acl = A - B K
            for (int i = 0; i < n; i++)
                for (int j = 0; j < n; j++) acl[i, j] = ad[i, j] - bd[i] * k[j];

            // P+ = Q + K'RK + Acl' P Acl  (上三角だけ計算して対称に書き戻す)
            var pAcl = Matrix.Multiply(p, acl);
            double diff = 0.0, magnitude = 0.0;
            var next = new double[n, n];
            for (int i = 0; i < n; i++)
                for (int j = i; j < n; j++)
                {
                    double s = 0.0;
                    for (int m = 0; m < n; m++) s += acl[m, i] * pAcl[m, j];
                    double v = (i == j ? qDiagonal[i] : 0.0) + r * k[i] * k[j] + s;
                    next[i, j] = v;
                    next[j, i] = v;
                    diff = Math.Max(diff, Math.Abs(v - p[i, j]));
                    magnitude = Math.Max(magnitude, Math.Abs(v));
                }

            p = next;
            if (diff <= RelativeTolerance * magnitude) break;
        }

        return k;
    }
}
