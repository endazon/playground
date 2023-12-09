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
            public bool AutoFontSizeAdjustment { get; set; } = false;
            #endregion

            #region BasicFigure
            protected override void Draw(Graphics g, IConvertTo? f = null)
            {
                var _Font      = AutoFontSizeAdjustment ? AdjustFontSize(g, Font) : Font;
                    _Font      = f is null ? Font      : f.ConvertToScale(Font     );
                var _Rectangle = f is null ? Rectangle : f.ConvertToScale(Rectangle);
                var _Format    = f is null ? Format    : f.ConvertToScale(Format   );
                g.DrawString(Text, _Font, Color, _Rectangle, _Format);
            }
            #endregion

            /// <summary>
            /// 引数で指定されたグラフィックオブジェクトに描画する最適なフォントサイズを算出します。
            /// </summary>
            /// <param name="str">出力する文字列</param>
            /// <param name="size">サイズ</param>
            /// <param name="g">グラフィックオブジェクト</param>
            /// <returns>フォントサイズ</returns>
            private Font AdjustFontSize(Graphics g, Font font)
            {
                var fontSize = font.Size;
                if (!string.IsNullOrEmpty(Text))
                {
                    var s1 = g.MeasureString(Text, new Font(font.Name, font.Size + 0.0f));
                    var s2 = g.MeasureString(Text, new Font(font.Name, font.Size + 0.1f));
                    var s  = new SizeF(s2.Width - s1.Width, s2.Height - s1.Height);
                    var a  = (Size.Width  / s.Width ) - 0.001f;
                    var b  = (Size.Height / s.Height) - 0.001f;
                    fontSize = (a < b) ? a : b;
                }
                return new(font.FontFamily, fontSize, font.Style, font.Unit);
            }


            public StringFigure() { }
            public StringFigure(PointF l, SizeF s, Brush c, string t, float ts) : base(l, s, c)
            {
                Text     = t;
                TextSize = ts;
            }
        }
    }
}