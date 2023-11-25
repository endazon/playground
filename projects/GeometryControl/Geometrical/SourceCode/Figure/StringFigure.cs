namespace Geometrical
{
    namespace Figure
    {
        public class StringFigure : BasicFigure, IStringFigure
        {
            #region ILineFigure
            public string Text { get; set; } = "";
            public float TextSize { get; set; } = 1.0f;
            public FontFamily FontFamily { get; set; } = new("MS UI Gothic");
            public FontStyle Style { get; set; } = FontStyle.Regular;
            public GraphicsUnit Unit { get; set; } = GraphicsUnit.Pixel;
            public StringFormat Format { get; set; } = new();
            public Font Font
            {
                get => new(FontFamily, TextSize, Style, Unit);
                set
                {
                    FontFamily = value.FontFamily;
                    TextSize   = value.Size      ;
                    Style      = value.Style     ;
                    Unit       = value.Unit      ;
                }
            }
            #endregion

            #region BasicFigure
            protected override void Draw(Graphics g, IConvertTo? f = null)
            {
                var _Font      = f is null ? Font      : f.ConvertToScale(Font     );
                var _Rectangle = f is null ? Rectangle : f.ConvertToScale(Rectangle);
                var _Format    = f is null ? Format    : f.ConvertToScale(Format   );
                g.DrawString(Text, _Font, Color, _Rectangle, _Format);
            }
            #endregion

            public StringFigure() { }
            public StringFigure(PointF l, SizeF s, Brush c, string t, float ts) : base(l, s, c)
            {
                Text     = t;
                TextSize = ts;
            }
        }
    }
}