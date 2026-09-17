namespace Geometrical
{
    namespace Figure
    {
        public class StringFigure : BasicStringFigure
        {
            #region BasicFigure
            protected override void Draw(Graphics g, IConvertTo? f = null)
            {
                var _Font      = AutoFontSizeAdjustment ? AdjustFontSize(g, Font) : Font;
                    _Font      = f is null ? _Font     : f.ConvertToScale(_Font    );
                var _Rectangle = f is null ? Rectangle : f.ConvertToScale(Rectangle);
                var _Format    = f is null ? Format    : f.ConvertToScale(Format   );
                g.DrawString(Text, _Font, Color, _Rectangle, _Format);
            }
            #endregion

            public StringFigure() { }
            public StringFigure(PointF l, SizeF s, Brush c, string t, float ts) : base(l, s, c, t, ts) { }
        }
    }
}