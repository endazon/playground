using Geometrical.Plane;

namespace Geometrical
{
    namespace Figure
    {
        public class CoordinateAxisDraw : FigureList
        {
            private StraightLineFigure CreateStraightLineFigure(PointF start, PointF end, Brush color)
            {
                var line = new StraightLineFigure(start, end, color, 3);

                return line;
            }
            private StringFigure CreateStringFigure(PointF location, SizeF size, string text, Brush color)
            {
                var str   = new StringFigure(location, size, color, text, 12.0f);
                str.Style = FontStyle.Bold;

                var format           = new StringFormat();
                format.Alignment     = StringAlignment.Center;
                format.LineAlignment = StringAlignment.Center;
                str.Format           = format;

                return str;
            }

            public CoordinateAxisDraw()
            {
                Add(CreateStringFigure(      new(60, 00), new(20, 20), "+X", Brushes.Red));
                Add(CreateStraightLineFigure(new(10, 10), new(60, 10),       Brushes.Red));
                Add(CreateStraightLineFigure(new(60, 10), new(50, 00),       Brushes.Red));
                Add(CreateStraightLineFigure(new(60, 10), new(50, 20),       Brushes.Red));
                Add(CreateStringFigure(      new(00, 60), new(20, 20), "+Y", Brushes.LimeGreen));
                Add(CreateStraightLineFigure(new(10, 10), new(10, 60),       Brushes.LimeGreen));
                Add(CreateStraightLineFigure(new(10, 60), new(00, 50),       Brushes.LimeGreen));
                Add(CreateStraightLineFigure(new(10, 60), new(20, 50),       Brushes.LimeGreen));
            }
        }
    }
}