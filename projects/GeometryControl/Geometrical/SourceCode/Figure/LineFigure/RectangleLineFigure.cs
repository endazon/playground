namespace Geometrical
{
    namespace Figure
    {
        public class RectangleLineFigure : BasicLineFigure
        {
            #region BasicFigure
            protected override void Draw(Graphics g, IConvertTo? f = null)
            {
                var _Pen       = f is null ? Pen       : f.ConvertToScale(Pen      );
                var _Rectangle = f is null ? Rectangle : f.ConvertToScale(Rectangle);
                g.DrawRectangle(_Pen, _Rectangle.X, _Rectangle.Y, _Rectangle.Width, _Rectangle.Height);
            }
            #endregion

            public RectangleLineFigure() { }
            public RectangleLineFigure(PointF l, SizeF s, Brush c, float ls) : base(l, s, c, ls) { }
        }
    }
}