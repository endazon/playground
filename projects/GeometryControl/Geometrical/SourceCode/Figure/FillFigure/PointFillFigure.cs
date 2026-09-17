namespace Geometrical
{
    namespace Figure
    {
        public class PointFillFigure : CircleFillFigure
        {
            public PointFillFigure() { }
            public PointFillFigure(PointF l, float s, Brush c) : base(new(l.X - s, l.Y - s), s * 2, c) { }
        }
    }
}