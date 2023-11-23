namespace Geometrical
{
    namespace Figure
    {
        public interface IConvertTo
        {
            float ConvertToScale(float value);
            PointF ConvertToScale(PointF value, PointF offset);
            PointF ConvertToScale(PointF value);
            SizeF ConvertToScale(SizeF value);
            RectangleF ConvertToScale(RectangleF value, PointF offset);
            RectangleF ConvertToScale(RectangleF value);
            Pen ConvertToScale(Pen value);
            Font ConvertToScale(Font value);
        }
        public interface IConvertFrom
        {
            float ConvertFromScale(float value);
            PointF ConvertFromScale(PointF value, PointF offset);
            PointF ConvertFromScale(PointF value);
            SizeF ConvertFromScale(SizeF value);
            RectangleF ConvertFromScale(RectangleF value, PointF offset);
            RectangleF ConvertFromScale(RectangleF value);
            Pen ConvertFromScale(Pen value);
            Font ConvertFromScale(Font value);
        }

        public interface IBasicFigure
        {
            bool Visible { get; set; }
            PointF Location { get; set; }
            SizeF Size { get; set; }
            RectangleF Rectangle { get; set; }
            Brush Color { get; set; }
            object? Tag { get; set; }

            void Drawing(Graphics g, IConvertTo? f = null);
        }
        public interface ILineFigure : IBasicFigure
        {
            float LineSize { get; set; }
            Pen Pen { get; set; }
        }
        public interface IStringFigure : IBasicFigure
        {
            string Text { get; set; }
            float TextSize { get; set; }
            FontFamily FontFamily { get; set; }
            FontStyle Style { get; set; }
            GraphicsUnit Unit { get; set; }
            StringFormat? Format { get; set; }
            Font Font { get; set; }
        }

        public abstract class BasicFigure : IBasicFigure
        {
            public virtual bool Visible { get; set; } = true;
            public virtual PointF Location { get; set; } = new();
            public virtual SizeF Size { get; set; } = new();
            public virtual RectangleF Rectangle
            {
                get => new(Location, Size);
                set
                {
                    Location = new(value.X, value.Y);
                    Size     = new(value.Width, value.Height);
                }
            }
            public virtual Brush Color { get; set; } = Brushes.White;
            public virtual object? Tag { get; set; } = null;

            protected abstract void Draw(Graphics g, IConvertTo? f = null);
            public void Drawing(Graphics g, IConvertTo? f = null)
            {
                if(Visible)
                {
                    Draw(g, f);
                }
            }

            public BasicFigure() { }
            public BasicFigure(PointF l, SizeF s, Brush c)
            {
                Location = l;
                Size     = s;
                Color    = c;
            }
        }
    }
}