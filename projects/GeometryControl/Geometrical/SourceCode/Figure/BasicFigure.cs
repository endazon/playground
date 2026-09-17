using System.Drawing;

namespace Geometrical
{
    namespace Figure
    {
        #region BasicFigure
        public abstract class BasicFigure : IFigure
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
            public string Name { get; set; } = "";
            public virtual object? Tag { get; set; } = null;

            public override string ToString() => Name;

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
        public abstract class BasicLineFigure : BasicFigure, ILineFigure
        {
            #region ILineFigure
            public virtual float LineSize { get; set; } = 1.0f;
            public virtual Pen Pen
            {
                get => new(Color, LineSize);
                set
                {
                    Color    = value.Brush;
                    LineSize = value.Width;
                }
            }
            #endregion

            public BasicLineFigure() { }
            public BasicLineFigure(PointF l, SizeF s, Brush c, float ls) : base(l, s, c)
            {
                LineSize = ls;
            }
        }        
        public abstract class BasicStringFigure : BasicFigure, IStringFigure
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

            /// <summary>
            /// 引数で指定されたグラフィックオブジェクトに描画する最適なフォントサイズを算出します。
            /// </summary>
            /// <param name="str">出力する文字列</param>
            /// <param name="size">サイズ</param>
            /// <param name="g">グラフィックオブジェクト</param>
            /// <returns>フォントサイズ</returns>
            protected Font AdjustFontSize(Graphics g, Font font)
            {
                var fontSize = font.Size;
                if (!string.IsNullOrEmpty(Text))
                {
                    var s1 = g.MeasureString(Text, new Font(font.Name, font.Size + 0.0f));
                    var s2 = g.MeasureString(Text, new Font(font.Name, font.Size + 1.0f));
                    var s  = new SizeF(s2.Width - s1.Width, s2.Height - s1.Height);
                    var a  = (Size.Width  / s.Width ) - 0.001f;
                    var b  = (Size.Height / s.Height) - 0.001f;
                    fontSize = (a < b) ? a : b;
                }
                return new(font.FontFamily, fontSize, font.Style, font.Unit);
            }

            public BasicStringFigure() { }
            public BasicStringFigure(PointF l, SizeF s, Brush c, string t, float ts) : base(l, s, c)
            {
                Text     = t;
                TextSize = ts;
            }
        }
        #endregion

        #region BasicPalygonFigure
        public abstract class BasicPalygonFigure : BasicFigure, IPolygonFigure
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
            #endregion

            public BasicPalygonFigure() { }
            public BasicPalygonFigure(PointF[] v, PointF l, SizeF s, Brush c) : base(l, s, c) { Vertex = v; }
        }
        public abstract class BasicPalygonLineFigure : BasicPalygonFigure, IPolygonLineFigure
        {
            #region ILineFigure
            public float LineSize { get; set; } = 1.0f;
            public Pen Pen
            {
                get => new(Color, LineSize);
                set
                {
                    Color = value.Brush;
                    LineSize = value.Width;
                }
            }
            #endregion

            public BasicPalygonLineFigure() { }
            public BasicPalygonLineFigure(PointF[] v, PointF l, SizeF s, Brush c, float ls) : base(v, l, s, c) 
            {
                LineSize = ls;
            }
        }
        #endregion

        #region TemplateFillAndLineAndStringFigure
        public class BasicTemplateFillAndLineAndStringFigure<FillClass, LineClass, StringClass> : BasicFigure, ITemplateFillAndLineAndStringFigure<FillClass, LineClass, StringClass>
            where FillClass   : IFigure      , new()
            where LineClass   : ILineFigure  , new()
            where StringClass : IStringFigure, new()
        {
            public FillClass Fill { get; set; } = new();
            public LineClass Line { get; set; } = new();
            public StringClass String { get; set; } = new();

            #region IFigure
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