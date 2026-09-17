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
    }
}