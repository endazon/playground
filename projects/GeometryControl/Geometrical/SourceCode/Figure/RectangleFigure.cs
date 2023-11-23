namespace Geometrical
{
    namespace Figure
    {
        public class RectangleFillFigure : BasicFigure
        {
            protected override void Draw(Graphics g, IConvertTo? f = null)
            {
                var _Rectangle = f is null ? Rectangle : f.ConvertToScale(Rectangle);
                g.FillRectangle(Color, _Rectangle);
            }
        }
        public class RectangleLineFigure : BasicFigure, ILineFigure
        {
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

            protected override void Draw(Graphics g, IConvertTo? f = null)
            {
                var _Pen       = f is null ? Pen       : f.ConvertToScale(Pen      );
                var _Rectangle = f is null ? Rectangle : f.ConvertToScale(Rectangle);
                g.DrawRectangle(_Pen, _Rectangle.X, _Rectangle.Y, _Rectangle.Width, _Rectangle.Height);
            }
        }
    }
}