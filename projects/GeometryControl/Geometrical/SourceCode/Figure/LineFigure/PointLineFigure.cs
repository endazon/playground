namespace Geometrical
{
    namespace Figure
    {
        public class PointLineFigure : CircleLineFigure
        {
            public PointLineFigure() { }
            public PointLineFigure(PointF l, float s, Brush c, float ls) : base(new(l.X - s , l.Y - s), s * 2, c, ls) { }
        }
    }
}