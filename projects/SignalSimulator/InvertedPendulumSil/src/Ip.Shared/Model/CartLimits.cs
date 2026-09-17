namespace Ip.Shared.Model;

/// <summary>
/// 台車のストローク方向の制限値。ソフトリミットと目標位置上限が独立にハードコードされて
/// 食い違うのを防ぐため、1 箇所から導出する。
/// </summary>
public static class CartLimits
{
    /// <summary>レール片側ストローク [m]。<see cref="PendulumParameters.RailStroke"/> の既定値。</summary>
    public const double RailStrokeM = 0.5;

    /// <summary>レール端から手前に取るソフトリミットの余裕 [m]</summary>
    public const double SoftLimitMarginM = 0.05;

    /// <summary>コントローラが FAULT に落とす台車位置 [m]</summary>
    public const double SoftLimitM = RailStrokeM - SoftLimitMarginM;

    /// <summary>
    /// 目標位置に許す上限とソフトリミットの間に確保する余裕 [m]。
    /// 目標へ整定する過程でオーバーシュートするため、上限をソフトリミットと同値にすると
    /// 「正規の最大値を入力しただけで必ず FAULT に落ちる」ことになる。
    /// </summary>
    public const double TargetOvershootMarginM = 0.05;

    /// <summary>台車の目標位置に指定できる絶対値の上限 [m]</summary>
    public const double MaxTargetM = SoftLimitM - TargetOvershootMarginM;
}
