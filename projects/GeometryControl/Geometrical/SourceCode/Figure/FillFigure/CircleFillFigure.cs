namespace Geometrical
{
    namespace Figure
    {
        public class CircleFillFigure : EllipseFillFigure
        {
            public CircleFillFigure() { }
            public CircleFillFigure(PointF l, float s, Brush c) : base(l, new(s, s), c) { }
        }
    }
}