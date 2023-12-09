using System.Drawing;

namespace Geometrical
{
    namespace Figure
    {
        #region IConvert
        public interface IConvertTo
        {
            public PointF ConvertToCoordinateDirection(PointF location);
            public SizeF ConvertToSizeDirection(SizeF size);
            float ConvertToScale(float value);
            PointF ConvertToScale(PointF value, PointF offset);
            PointF ConvertToScale(PointF value);
            SizeF ConvertToScale(SizeF value);
            RectangleF ConvertToScale(RectangleF value, PointF offset);
            RectangleF ConvertToScale(RectangleF value);
            Pen ConvertToScale(Pen value);
            Font ConvertToScale(Font value);
            StringFormat ConvertToScale(StringFormat value);
        }
        public interface IConvertFrom
        {
            public PointF ConvertFromCoordinateDirection(PointF location);
            public SizeF ConvertFromSizeDirection(SizeF size);
            float ConvertFromScale(float value);
            PointF ConvertFromScale(PointF value, PointF offset);
            PointF ConvertFromScale(PointF value);
            SizeF ConvertFromScale(SizeF value);
            RectangleF ConvertFromScale(RectangleF value, PointF offset);
            RectangleF ConvertFromScale(RectangleF value);
            Pen ConvertFromScale(Pen value);
            Font ConvertFromScale(Font value);
            StringFormat ConvertFromScale(StringFormat value);
        }
        #endregion

        #region BasicFigure
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
            StringFormat Format { get; set; }
            Font Font { get; set; }
            bool AutoFontSizeAdjustment { get; set; }
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
        #endregion

        #region BasePalygonFigure
        public interface IPolygonFigure : IBasicFigure
        {
            PointF[] Vertex { get; set; }
        }
        public interface IPolygonLineFigure : IPolygonFigure, ILineFigure { }
        public abstract class BasePalygonFigure : BasicFigure, IPolygonFigure
        {
            #region IPolygonFigure
            private PointF[] _Vertex = new PointF[0];
            public virtual PointF[] Vertex
            {
                get => _Vertex;
                set
                {
                    _Vertex = value;
                    float L = 0, T = 0, R = 0, B = 0;
                    for (int i = 0; i < _Vertex.Length; i++)
                    {
                        var point = _Vertex[i];
                        if (i == 0)
                        {
                            L = point.X;
                            T = point.Y;
                            R = point.X;
                            B = point.Y;
                        }
                        else
                        {
                            L = point.X < L ? point.X : L;
                            T = point.Y < T ? point.Y : T;
                            R = point.X > R ? point.X : R;
                            B = point.Y > B ? point.Y : B;
                        }
                    }
                    base.Location = new(             L ,              T );
                    base.Size     = new(Math.Abs(R - L), Math.Abs(B - T));
                }
            }
            #endregion

            #region BasicFigure
            public override PointF Location
            {
                get => base.Location;
                set
                {
                    var offset = new PointF(value.X - base.Location.X, value.Y - base.Location.Y);
                    var vertex = new PointF[Vertex.Length];
                    for (int i = 0; i < vertex.Length; i++)
                    {
                        vertex[i] = new(
                            /* X = */Vertex[i].X + offset.X,
                            /* Y = */Vertex[i].Y + offset.Y
                            );
                    }
                    _Vertex = vertex;
                    base.Location = value;
                }
            }
            public override SizeF Size
            {
                get => base.Size;
                set
                {
                    var deformationRate = new SizeF(value.Width / base.Size.Width, value.Height / base.Size.Height);
                    if (deformationRate.Width != float.NaN && deformationRate.Height != float.NaN)
                    {
                        var vertex = new PointF[Vertex.Length];
                        for (int i = 0; i < vertex.Length; i++)
                        {
                            vertex[i] = new(
                                /* X = */Location.X + ((Vertex[i].X - Location.X) * deformationRate.Width),
                                /* Y = */Location.Y + ((Vertex[i].Y - Location.Y) * deformationRate.Height)
                                );
                        }
                        _Vertex = vertex;
                    }
                    base.Size = value;
                }
            }

            public BasePalygonFigure() { }
            public BasePalygonFigure(PointF[] v, PointF l, SizeF s, Brush c) : base(l, s, c) { Vertex = v; }
            #endregion
        }
        #endregion

        #region TemplateFillAndLineAndStringFigure
        public interface ITemplateFillAndLineAndStringFigure<FillClass, LineClass, StringClass> : IBasicFigure
        {
            FillClass Fill { get; set; }
            LineClass Line { get; set; }
            StringClass String { get; set; }
        }        
        public class BasicTemplateFillAndLineAndStringFigure<FillClass, LineClass, StringClass> : BasicFigure, ITemplateFillAndLineAndStringFigure<FillClass, LineClass, StringClass>
            where FillClass   : IBasicFigure , new()
            where LineClass   : ILineFigure  , new()
            where StringClass : IStringFigure, new()
        {
            public FillClass Fill { get; set; } = new();
            public LineClass Line { get; set; } = new();
            public StringClass String { get; set; } = new();

            #region IBasicFigure
            public override bool Visible
            {
                get => base.Visible;
                set
                {
                    base.Visible = value;
                    Fill.Visible = value;
                    Line.Visible = value;
                    String.Visible = value;
                }
            }
            public override PointF Location
            {
                get => base.Location;
                set
                {
                    base.Location = value;
                    Fill.Location = value;
                    Line.Location = value;
                    String.Location = value;
                }
            }
            public override SizeF Size
            {
                get => base.Size;
                set
                {
                    base.Size = value;
                    Fill.Size = value;
                    Line.Size = value;
                    String.Size = value;
                }
            }
            public override RectangleF Rectangle
            {
                get => new(Location, Size);
                set
                {
                    Location = new(value.X, value.Y);
                    Size = new(value.Width, value.Height);
                }
            }
            public override Brush Color
            {
                get => Fill.Color;
                set => Fill.Color = value;
            }

            protected override void Draw(Graphics g, IConvertTo? f = null)
            {
                Fill.Drawing(g, f);
                Line.Drawing(g, f);
                String.Drawing(g, f);
            }
            #endregion

            public BasicTemplateFillAndLineAndStringFigure() { }
            public BasicTemplateFillAndLineAndStringFigure(PointF l, SizeF s, Brush c, float ls, string t, float ts) 
            {
                Location        = l;
                Size            = s;
                Color           = c;
                Line.Color      = Brushes.Black;
                Line.LineSize   = ls;
                String.Color    = Brushes.Black;
                String.Text     = t;
                String.TextSize = ts;
            }
        }
        public class BasicTemplatePolygonFillAndLineAndStringFigure<FillClass, LineClass, StringClass> : BasicTemplateFillAndLineAndStringFigure<FillClass, LineClass, StringClass>, IPolygonFigure
            where FillClass   : IPolygonFigure    , new()
            where LineClass   : IPolygonLineFigure, new()
            where StringClass : IStringFigure     , new()
        {
            #region IPolygonFigure
            private PointF[] _Vertex = new PointF[0];
            public PointF[] Vertex
            {
                get => _Vertex;
                set
                {
                    _Vertex = value;
                    float L = 0, T = 0, R = 0, B = 0;
                    for (int i = 0; i < _Vertex.Length; i++)
                    {
                        var point = _Vertex[i];
                        if (i == 0)
                        {
                            L = point.X;
                            T = point.Y;
                            R = point.X;
                            B = point.Y;
                        }
                        else
                        {
                            L = point.X < L ? point.X : L;
                            T = point.Y < T ? point.Y : T;
                            R = point.X > R ? point.X : R;
                            B = point.Y > B ? point.Y : B;
                        }
                    }
                    base.Location = new(             L ,              T );
                    base.Size     = new(Math.Abs(R - L), Math.Abs(B - T));

                    Fill.Vertex = _Vertex;
                    Line.Vertex = _Vertex;
                    String.Rectangle = Rectangle;
                }
            }
            #endregion

            public BasicTemplatePolygonFillAndLineAndStringFigure() { }
            public BasicTemplatePolygonFillAndLineAndStringFigure(PointF[] v, PointF l, SizeF s, Brush c, float ls, string t, float ts) : base(l, s, c, ls, t, ts)
            {
                Vertex = v;
            }
        }
        #endregion
    }
}