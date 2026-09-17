namespace Geometrical
{
    namespace Figure
    {
        public class SquareLineFigure : RectangleLineFigure
        {
            public SquareLineFigure() { }
            public SquareLineFigure(PointF l, float s, Brush c, float ls) : base(l, new(s, s), c, ls) { }
        }
    }
}