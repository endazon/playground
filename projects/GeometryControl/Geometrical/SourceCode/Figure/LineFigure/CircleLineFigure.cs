namespace Geometrical
{
    namespace Figure
    {
        public class CircleLineFigure : EllipseLineFigure
        {
            public CircleLineFigure() { }
            public CircleLineFigure(PointF l, float s, Brush c, float ls) : base(l, new(s, s), c, ls) { }
        }
    }
}