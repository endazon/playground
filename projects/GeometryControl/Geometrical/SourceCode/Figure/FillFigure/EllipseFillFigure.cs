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
    }
}