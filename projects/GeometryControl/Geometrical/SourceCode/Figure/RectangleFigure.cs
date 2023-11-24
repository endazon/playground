namespace Geometrical
{
    namespace Figure
    {
        public class RectangleFillFigure : BasicFigure
        {
            #region BasicFigure
            protected override void Draw(Graphics g, IConvertTo? f = null)
            {
                var _Rectangle = f is null ? Rectangle : f.ConvertToScale(Rectangle);
                g.FillRectangle(Color, _Rectangle);
            }
            #endregion

            public RectangleFillFigure() { }
            public RectangleFillFigure(PointF l, SizeF s, Brush c) : base(l, s, c) { }
        }
        public class RectangleLineFigure : BasicFigure, ILineFigure
        {
            #region ILineFigure
            public float LineSize { get; set; } = 1.0f;
            public Pen Pen
            {
                get => new(Color, LineSize);
                set
                {
                    Color    = value.Brush;
                    LineSize = value.Width;
                }
            }
            #endregion

            #region BasicFigure
            protected override void Draw(Graphics g, IConvertTo? f = null)
            {
                var _Pen       = f is null ? Pen       : f.ConvertToScale(Pen      );
                var _Rectangle = f is null ? Rectangle : f.ConvertToScale(Rectangle);
                g.DrawRectangle(_Pen, _Rectangle.X, _Rectangle.Y, _Rectangle.Width, _Rectangle.Height);
            }
            #endregion

            public RectangleLineFigure() { }
            public RectangleLineFigure(PointF l, SizeF s, Brush c, float ls) : base(l, s, c)
            {
                LineSize = ls;
            }
        }

        public class SquareFillFigure : RectangleFillFigure
        {
            public SquareFillFigure() { }
            public SquareFillFigure(PointF l, float s, Brush c) : base(l, new(s, s), c) { }
        }
        public class SquareLineFigure : RectangleLineFigure
        {
            public SquareLineFigure() { }
            public SquareLineFigure(PointF l, float s, Brush c, float ls) : base(l, new(s, s), c, ls) { }
        }
    }
}