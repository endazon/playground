namespace Geometrical
{
    namespace Figure
    {
        public class SquareFillFigure : RectangleFillFigure
        {
            public SquareFillFigure() { }
            public SquareFillFigure(PointF l, float s, Brush c) : base(l, new(s, s), c) { }
        }
    }
}