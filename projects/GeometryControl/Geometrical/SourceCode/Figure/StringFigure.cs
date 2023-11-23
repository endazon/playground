namespace Geometrical
{
    namespace Figure
    {
        public class StringFigure : BasicFigure, IStringFigure
        {
            public virtual string Text { get; set; } = "";
            public virtual float TextSize { get; set; } = 1.0f;
            public virtual FontFamily FontFamily { get; set; } = new("MS UI Gothic");
            public virtual FontStyle Style { get; set; } = FontStyle.Regular;
            public virtual GraphicsUnit Unit { get; set; } = GraphicsUnit.Pixel;
            public virtual StringFormat? Format { get; set; } = null;
            public virtual Font Font
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

            protected override void Draw(Graphics g, IConvertTo? f = null)
            {
                var _Font      = f is null ? Font      : f.ConvertToScale(Font     );
                var _Rectangle = f is null ? Rectangle : f.ConvertToScale(Rectangle);
                g.DrawString(Text, _Font, Color, _Rectangle, Format);
            }
        }
    }
}