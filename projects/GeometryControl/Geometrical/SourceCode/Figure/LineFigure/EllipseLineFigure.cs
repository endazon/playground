namespace Geometrical
{
    namespace Figure
    {
        public class EllipseLineFigure : BasicLineFigure
        {
            #region BasicFigure
            protected override void Draw(Graphics g, IConvertTo? f = null)
            {
                var _Pen       = f is null ? Pen       : f.ConvertToScale(Pen      );
                var _Rectangle = f is null ? Rectangle : f.ConvertToScale(Rectangle);
                g.DrawEllipse(_Pen, _Rectangle.X, _Rectangle.Y, _Rectangle.Width, _Rectangle.Height);
            }
            #endregion

            public EllipseLineFigure() { }
            public EllipseLineFigure(PointF l, SizeF s, Brush c, float ls) : base(l, s, c, ls) { }
        }
    }
}