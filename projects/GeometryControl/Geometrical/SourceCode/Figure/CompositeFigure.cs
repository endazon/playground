namespace Geometrical
{
    namespace Figure
    {
        public class TmpCompositeFigure<FillClass, LineClass, StringClass> : BasicFigure
            where FillClass   : IBasicFigure , new()
            where LineClass   : ILineFigure  , new()
            where StringClass : IStringFigure, new()
        {
            public FillClass Fill { get; set; } = new();
            public LineClass Line { get; set; } = new();
            public StringClass String { get; set; } = new();

            #region BasicFigure
            public override bool Visible
            {
                get => base.Visible;
                set
                {
                    base.Visible   = value;
                    Fill.Visible   = base.Visible;
                    Line.Visible   = base.Visible;
                    String.Visible = base.Visible;
                }
            }
            public override PointF Location
            {
                get => base.Location;
                set
                {
                    base.Location   = value;
                    Fill.Location   = base.Location;
                    Line.Location   = base.Location;
                    String.Location = base.Location;
                }
            }
            public override SizeF Size
            {
                get => base.Size;
                set
                {
                    base.Size   = value;
                    Fill.Size   = base.Size;
                    Line.Size   = base.Size;
                    String.Size = base.Size;
                }
            }

            protected override void Draw(Graphics g, IConvertTo? f = null)
            {
                Fill.Drawing(g, f);
                Line.Drawing(g, f);
                String.Drawing(g, f);
            }
            #endregion

            public TmpCompositeFigure() { }
            public TmpCompositeFigure(PointF l, SizeF s, Brush c, float ls, string t, float ts) : base(l, s, c)
            {
                Line.LineSize   = ls;
                String.Text     = t;
                String.TextSize = ts;
            }
        }

        public class CoordinateAxisDraw : FigureList
        {
            private StringFigure CreateStringFigure(PointF location, SizeF size, string text, Brush color)
            {
                var str = new StringFigure(location, size, color, text, 12.0f);
                str.Style = FontStyle.Bold;

                var format = new StringFormat();
                format.Alignment = StringAlignment.Center;
                format.LineAlignment = StringAlignment.Center;
                str.Format = format;

                return str;
            }
            private StraightLineFigure CreateStraightLineFigure(PointF start, PointF end, Brush color)
            {
                var line = new StraightLineFigure(start, end, color, 3);

                return line;
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