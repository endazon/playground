namespace Geometrical
{
    namespace Figure
    {
        public class EllipseFillFigure : BasicFigure
        {
            #region BasicFigure
            protected override void Draw(Graphics g, IConvertTo? f = null)
            {
                var _Rectangle = f is null ? Rectangle : f.ConvertToScale(Rectangle);
                g.FillEllipse(Color, _Rectangle);
            }
            #endregion

            public EllipseFillFigure() { }
            public EllipseFillFigure(PointF l, SizeF s, Brush c) : base(l, s, c) { }
        }
        public class EllipseLineFigure : BasicFigure, ILineFigure
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
                g.DrawEllipse(_Pen, _Rectangle.X, _Rectangle.Y, _Rectangle.Width, _Rectangle.Height);
            }
            #endregion

            public EllipseLineFigure() { }
            public EllipseLineFigure(PointF l, SizeF s, Brush c, float ls) : base(l, s, c)
            {
                LineSize = ls;
            }
        }

        public class CircleFillFigure : EllipseFillFigure
        {
            public CircleFillFigure() { }
            public CircleFillFigure(PointF l, float s, Brush c) : base(l, new(s, s), c) { }
        }
        public class CircleLineFigure : EllipseLineFigure
        {
            public CircleLineFigure() { }
            public CircleLineFigure(PointF l, float s, Brush c, float ls) : base(l, new(s, s), c, ls) { }
        }

        public class PointFillFigure : CircleFillFigure
        {
            public PointFillFigure() { }
            public PointFillFigure(PointF l, float s, Brush c) : base(new(l.X - s, l.Y - s), s * 2, c) { }
        }
        public class PointLineFigure : CircleLineFigure
        {
            public PointLineFigure() { }
            public PointLineFigure(PointF l, float s, Brush c, float ls) : base(new(l.X - s , l.Y - s), s * 2, c, ls) { }
        }
    }
}