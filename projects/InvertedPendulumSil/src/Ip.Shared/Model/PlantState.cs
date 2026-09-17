namespace Ip.Shared.Model;

/// <summary>
/// プラント状態 [x, θ, ẋ, θ̇]。θ=0 が倒立、θ が正の向きは +x 側へ倒れる向き。
/// </summary>
public readonly record struct PlantState(double X, double Theta, double XDot, double ThetaDot)
{
    public static PlantState Hanging => new(0.0, Math.PI, 0.0, 0.0);
    public static PlantState NearUpright(double thetaOffset = 0.02) => new(0.0, thetaOffset, 0.0, 0.0);

    public static PlantState operator +(PlantState a, PlantState b)
        => new(a.X + b.X, a.Theta + b.Theta, a.XDot + b.XDot, a.ThetaDot + b.ThetaDot);

    public static PlantState operator *(PlantState a, double k)
        => new(a.X * k, a.Theta * k, a.XDot * k, a.ThetaDot * k);

    public double this[int i] => i switch
    {
        0 => X,
        1 => Theta,
        2 => XDot,
        3 => ThetaDot,
        _ => throw new ArgumentOutOfRangeException(nameof(i)),
    };

    public double[] ToArray() => [X, Theta, XDot, ThetaDot];

    public static PlantState FromArray(ReadOnlySpan<double> v) => new(v[0], v[1], v[2], v[3]);
}

/// <summary>外乱入力。Force は台車への水平力 [N]、Torque は振子軸まわりのトルク [Nm]。</summary>
public readonly record struct Disturbance(double Force, double Torque)
{
    public static readonly Disturbance None = new(0.0, 0.0);
}
