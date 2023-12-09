using Geometrical.Plane;

namespace Geometrical
{
    namespace Figure
    {
        public class RectangleFigure : BasicTemplateFillAndLineAndStringFigure<RectangleFillFigure, RectangleLineFigure, StringFigure>
        {
            public RectangleFigure() { }
            public RectangleFigure(PointF l, SizeF s, Brush c, float ls, string t, float ts) : base(l, s, c, ls, t, ts) { }
        }
        public class SquareFigure : BasicTemplateFillAndLineAndStringFigure<SquareFillFigure, SquareLineFigure, StringFigure>
        {
            public SquareFigure() { }
            public SquareFigure(PointF l, float s, Brush c, float ls, string t, float ts) : base(l, new(s, s), c, ls, t, ts) { }
        }
        public class EllipseFigure : BasicTemplateFillAndLineAndStringFigure<EllipseFillFigure, EllipseLineFigure, StringFigure>
        {
            public EllipseFigure() { }
            public EllipseFigure(PointF l, SizeF s, Brush c, float ls, string t, float ts) : base(l, s, c, ls, t, ts) { }
        }
        public class CircleFigure : BasicTemplateFillAndLineAndStringFigure<CircleFillFigure, CircleLineFigure, StringFigure>
        {
            public CircleFigure() { }
            public CircleFigure(PointF l, float s, Brush c, float ls, string t, float ts) : base(l, new(s, s), c, ls, t, ts) { }
        }
        public class PointFigure : BasicTemplateFillAndLineAndStringFigure<PointFillFigure, PointLineFigure, StringFigure>
        {
            public PointFigure() { }
            public PointFigure(PointF l, float s, Brush c, float ls, string t, float ts) : base(new(l.X - s, l.Y - s), new(s * 2, s * 2), c, ls, t, ts) { }
        }

        public class PolygonFigure : BasicTemplatePolygonFillAndLineAndStringFigure<PolygonFillFigure, PolygonLineFigure, StringFigure>
        {
            public PolygonFigure() { }
            public PolygonFigure(PointF[] v, PointF l, SizeF s, Brush c, float ls, string t, float ts) : base(v, l, s, c, ls, t, ts) { }
        }

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
        public class CoordinateGridDraw : FigureList
        {
            private PointFillFigure CreatePointFigure(PointF location, float size, Brush color)
            {
                var point = new PointFillFigure(location, size, color);

                return point;
            }
            private StraightLineFigure CreateStraightLineFigure(PointF start, PointF end, Brush color, float lineSize)
            {
                var line = new StraightLineFigure(start, end, color, lineSize);

                return line;
            }
#if false//デバッグ用
            private RectangleFigure CreateStringFigure(PointF location, SizeF size, string text, float textSize, Brush color, StringAlignment alignment, StringAlignment lineAlignment)
            {
                var str   = new RectangleFigure();

                var format           = new StringFormat();
                format.Alignment     = alignment;
                format.LineAlignment = lineAlignment;

                str.Location         = location;
                str.Size             = size;
                str.Fill.Color       = Brushes.Transparent;
                str.Line.Color       = Brushes.Cyan;
                str.Line.LineSize    = 0.1f;
                str.String.Color     = color;
                str.String.Text      = text;
                str.String.TextSize  = textSize;
                str.String.Format    = format;
                str.String.Style     = FontStyle.Bold;

                return str;
            }
#else
            private StringFigure CreateStringFigure(PointF location, SizeF size, string text, float textSize, Brush color, StringAlignment alignment, StringAlignment lineAlignment)
            {
                var str   = new StringFigure(location, size, color, text, textSize);
                str.Style = FontStyle.Bold;

                var format           = new StringFormat();
                format.Alignment     = alignment;
                format.LineAlignment = lineAlignment;
                str.Format           = format;

                return str;
            }
#endif

            #region BasicFigure
            public override Brush Color { get => Brushes.Transparent; }
            #endregion

            public void UpdateGrid(Size areaSize, CoordinateSystem system, uint breakInterval)
            {
                var origin       = system.Origin;
                var scale        = system.ReducedScale;
                var separat      = breakInterval * scale;
                var _areaSize    = system.ConvertToSizeDirection(areaSize);
                var left         = -origin.X;
                var top          = -origin.Y;
                var right        = left + system.ConvertFromScale(_areaSize.Width);
                var bottom       = top  + system.ConvertFromScale(_areaSize.Height);
                var lineSize     = system.ConvertFromScale(1.0f);
                var strLocationO = new PointF(system.ConvertFromScale(-16), system.ConvertFromScale(-16));
                var strSizeO     = system.ConvertFromScale(new SizeF(16f, 16f));
                var strSize      = system.ConvertFromScale(new SizeF(40f, 16f));
                var fontSize     = system.ConvertFromScale(12.0f);

                Clear();

                /*グリッド*/
                //X軸 正の方向
                for (var pos = left < 0 ? +1 : Convert.ToInt32(left); pos < right; pos++)
                {
                    if ((pos % scale) != 0) { continue; }
                    Add(CreateStraightLineFigure(new(pos, top), new(pos, bottom), (pos % separat) == 0 ? Brushes.DarkGray : Brushes.LightGray, lineSize));
                }
                //X軸 負の方向
                for (var pos = 0 < right ? -1 : Convert.ToInt32(right); left < pos; pos--)
                {
                    if ((pos % scale) != 0) { continue; }
                    Add(CreateStraightLineFigure(new(pos, top), new(pos, bottom), (pos % separat) == 0 ? Brushes.DarkGray : Brushes.LightGray, lineSize));
                }
                //Y軸 正の方向
                for (var pos = top < 0 ? +1 : Convert.ToInt32(top); pos < bottom; pos++)
                {
                    if ((pos % scale) != 0) { continue; }
                    Add(CreateStraightLineFigure(new(left, pos), new(right, pos), (pos % separat) == 0 ? Brushes.DarkGray : Brushes.LightGray, lineSize));
                }
                //Y軸 負の方向
                for (var pos = 0 < bottom ? -1 : Convert.ToInt32(bottom); top < pos; pos--)
                {
                    if ((pos % scale) != 0) { continue; }
                    Add(CreateStraightLineFigure(new(left, pos), new(right, pos), (pos % separat) == 0 ? Brushes.DarkGray : Brushes.LightGray, lineSize));
                }

                /*原点*/
                Add(CreatePointFigure(new(0, 0), system.ConvertFromScale(2.5f), Brushes.Black));
                Add(CreateStraightLineFigure(new(0, top), new(0, bottom), Brushes.Black, lineSize));
                Add(CreateStraightLineFigure(new(left, 0), new(right, 0), Brushes.Black, lineSize));

                /*文字表示*/
                Add(CreateStringFigure(strLocationO, strSizeO, "O", fontSize, Brushes.Black, StringAlignment.Far, StringAlignment.Far));
                //X軸 正の方向
                for (var pos = left < 0 ? +1 : Convert.ToInt32(left); pos < right; pos++)
                {
                    if ((pos % scale  ) != 0) { continue; }
                    if ((pos % separat) != 0) { continue; }
                    if (0 < top + strSize.Height)
                    {
                        Add(CreateStringFigure(new(pos - strSize.Width / 2, top                 ), strSize, (Convert.ToInt32(pos / separat) * separat).ToString(), fontSize, Brushes.Black, StringAlignment.Center, StringAlignment.Far));
                    }
                    else if (bottom < 0)
                    {
                        Add(CreateStringFigure(new(pos - strSize.Width / 2, bottom - strSize.Height), strSize, (Convert.ToInt32(pos / separat) * separat).ToString(), fontSize, Brushes.Black, StringAlignment.Center, StringAlignment.Far));
                    }
                    else
                    {
                        Add(CreateStringFigure(new(pos - strSize.Width / 2, -strSize.Height        ), strSize, (Convert.ToInt32(pos / separat) * separat).ToString(), fontSize, Brushes.Black, StringAlignment.Center, StringAlignment.Far));
                    }
                }
                //X軸 負の方向
                for (var pos = 0 < right ? -1 : Convert.ToInt32(right); left < pos; pos--)
                {
                    if ((pos % scale  ) != 0) { continue; }
                    if ((pos % separat) != 0) { continue; }
                    if (0 < top + strSize.Height)
                    {
                        Add(CreateStringFigure(new(pos - strSize.Width / 2, top                 ), strSize, (Convert.ToInt32(pos / separat) * separat).ToString(), fontSize, Brushes.Black, StringAlignment.Center, StringAlignment.Far));
                    }
                    else if (bottom < 0)
                    {
                        Add(CreateStringFigure(new(pos - strSize.Width / 2, bottom - strSize.Height), strSize, (Convert.ToInt32(pos / separat) * separat).ToString(), fontSize, Brushes.Black, StringAlignment.Center, StringAlignment.Far));
                    }
                    else
                    {
                        Add(CreateStringFigure(new(pos - strSize.Width / 2, -strSize.Height        ), strSize, (Convert.ToInt32(pos / separat) * separat).ToString(), fontSize, Brushes.Black, StringAlignment.Center, StringAlignment.Far));
                    }
                }
                //Y軸 正の方向
                for (var pos = top < 0 ? +1 : Convert.ToInt32(top); pos < bottom; pos++)
                {
                    if ((pos % scale  ) != 0) { continue; }
                    if ((pos % separat) != 0) { continue; }
                    if (0 < left + strSize.Width)
                    {
                        Add(CreateStringFigure(new(left - (strSize.Width - strSize.Width), pos - strSize.Height / 2), strSize, (Convert.ToInt32(pos / separat) * separat).ToString(), fontSize, Brushes.Black, StringAlignment.Far, StringAlignment.Center));
                    }
                    else if (right < 0)
                    {
                        Add(CreateStringFigure(new(right - strSize.Width              , pos - strSize.Height / 2), strSize, (Convert.ToInt32(pos / separat) * separat).ToString(), fontSize, Brushes.Black, StringAlignment.Far, StringAlignment.Center));
                    }
                    else
                    {
                        Add(CreateStringFigure(new(-strSize.Width                     , pos - strSize.Height / 2), strSize, (Convert.ToInt32(pos / separat) * separat).ToString(), fontSize, Brushes.Black, StringAlignment.Far, StringAlignment.Center));
                    }
                }
                //Y軸 負の方向
                for (var pos = 0 < bottom ? -1 : Convert.ToInt32(bottom); top < pos; pos--)
                {
                    if ((pos % scale  ) != 0) { continue; }
                    if ((pos % separat) != 0) { continue; }
                    if (0 < left + strSize.Width)
                    {
                        Add(CreateStringFigure(new(left - (strSize.Width - strSize.Width), pos - strSize.Height / 2), strSize, (Convert.ToInt32(pos / separat) * separat).ToString(), fontSize, Brushes.Black, StringAlignment.Far, StringAlignment.Center));
                    }
                    else if (right < 0)
                    {
                        Add(CreateStringFigure(new(right - strSize.Width              , pos - strSize.Height / 2), strSize, (Convert.ToInt32(pos / separat) * separat).ToString(), fontSize, Brushes.Black, StringAlignment.Far, StringAlignment.Center));
                    }
                    else
                    {
                        Add(CreateStringFigure(new(-strSize.Width                     , pos - strSize.Height / 2), strSize, (Convert.ToInt32(pos / separat) * separat).ToString(), fontSize, Brushes.Black, StringAlignment.Far, StringAlignment.Center));
                    }
                }
            }

            public CoordinateGridDraw() { }
        }
    }
}