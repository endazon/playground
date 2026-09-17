namespace Ip.Shared.Numerics;

/// <summary>
/// 制御設計に必要な最小限の密行列演算。外部依存を持たず、Blazor WASM でもそのまま動くことを優先している。
/// 行列は double[n][] のジャグ配列ではなく double[n, n] で扱う。
/// </summary>
public static class Matrix
{
    public static double[,] Identity(int n)
    {
        var m = new double[n, n];
        for (int i = 0; i < n; i++) m[i, i] = 1.0;
        return m;
    }

    public static double[,] Multiply(double[,] a, double[,] b)
    {
        int n = a.GetLength(0), k = a.GetLength(1), m = b.GetLength(1);
        if (b.GetLength(0) != k) throw new ArgumentException("次元が一致しません。", nameof(b));
        var r = new double[n, m];
        for (int i = 0; i < n; i++)
            for (int p = 0; p < k; p++)
            {
                double aip = a[i, p];
                if (aip == 0.0) continue;
                for (int j = 0; j < m; j++) r[i, j] += aip * b[p, j];
            }
        return r;
    }

    public static double[] Multiply(double[,] a, double[] x)
    {
        int n = a.GetLength(0), k = a.GetLength(1);
        if (x.Length != k) throw new ArgumentException("次元が一致しません。", nameof(x));
        var r = new double[n];
        for (int i = 0; i < n; i++)
        {
            double s = 0.0;
            for (int j = 0; j < k; j++) s += a[i, j] * x[j];
            r[i] = s;
        }
        return r;
    }

    /// <summary>最大絶対行和ノルム。</summary>
    public static double InfinityNorm(double[,] a)
    {
        int n = a.GetLength(0), m = a.GetLength(1);
        double best = 0.0;
        for (int i = 0; i < n; i++)
        {
            double s = 0.0;
            for (int j = 0; j < m; j++) s += Math.Abs(a[i, j]);
            if (s > best) best = s;
        }
        return best;
    }

    /// <summary>
    /// 行列指数 exp(M)。scaling &amp; squaring + 14 次テイラー。
    /// 本システムの行列は 5x5 以下・ノルムが小さいため、この単純な実装で十分な精度が出る。
    /// </summary>
    public static double[,] Exp(double[,] m)
    {
        int n = m.GetLength(0);
        double norm = InfinityNorm(m);
        int squarings = Math.Max(0, (int)Math.Ceiling(Math.Log2(norm <= 0.0 ? 1.0 : norm)) + 2);
        double scale = Math.Pow(2.0, squarings);

        var s = new double[n, n];
        for (int i = 0; i < n; i++)
            for (int j = 0; j < n; j++) s[i, j] = m[i, j] / scale;

        var result = Identity(n);
        var term = Identity(n);
        for (int k = 1; k <= 14; k++)
        {
            term = Multiply(term, s);
            for (int i = 0; i < n; i++)
                for (int j = 0; j < n; j++)
                {
                    term[i, j] /= k;
                    result[i, j] += term[i, j];
                }
        }
        for (int i = 0; i < squarings; i++) result = Multiply(result, result);
        return result;
    }
}

/// <summary>連続時間状態空間 (単入力) を零次ホールドで離散化した結果。</summary>
public readonly record struct DiscreteSystem(double[,] Ad, double[] Bd);

public static class Discretization
{
    /// <summary>
    /// Van Loan 法による零次ホールド離散化。
    /// [[A, B], [0, 0]] * Ts の行列指数の左上・右上ブロックがそのまま Ad, Bd になる。
    /// </summary>
    public static DiscreteSystem ZeroOrderHold(double[,] a, double[] b, double tsSeconds)
    {
        int n = a.GetLength(0);
        var block = new double[n + 1, n + 1];
        for (int i = 0; i < n; i++)
        {
            for (int j = 0; j < n; j++) block[i, j] = a[i, j] * tsSeconds;
            block[i, n] = b[i] * tsSeconds;
        }

        var e = Matrix.Exp(block);
        var ad = new double[n, n];
        var bd = new double[n];
        for (int i = 0; i < n; i++)
        {
            for (int j = 0; j < n; j++) ad[i, j] = e[i, j];
            bd[i] = e[i, n];
        }
        return new DiscreteSystem(ad, bd);
    }
}
