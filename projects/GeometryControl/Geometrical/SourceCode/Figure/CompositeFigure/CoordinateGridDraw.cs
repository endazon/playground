using Geometrical.Plane;

namespace Geometrical
{
    namespace Figure
    {
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
                        Add(CreateStringFigure(new(pos - strSize.Width / 2, top                    ), strSize, (Convert.ToInt32(pos / separat) * separat).ToString(), fontSize, Brushes.Black, StringAlignment.Center, StringAlignment.Far));
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
                        Add(CreateStringFigure(new(pos - strSize.Width / 2, top                    ), strSize, (Convert.ToInt32(pos / separat) * separat).ToString(), fontSize, Brushes.Black, StringAlignment.Center, StringAlignment.Far));
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
                        Add(CreateStringFigure(new(right - strSize.Width                 , pos - strSize.Height / 2), strSize, (Convert.ToInt32(pos / separat) * separat).ToString(), fontSize, Brushes.Black, StringAlignment.Far, StringAlignment.Center));
                    }
                    else
                    {
                        Add(CreateStringFigure(new(-strSize.Width                        , pos - strSize.Height / 2), strSize, (Convert.ToInt32(pos / separat) * separat).ToString(), fontSize, Brushes.Black, StringAlignment.Far, StringAlignment.Center));
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
                        Add(CreateStringFigure(new(right - strSize.Width                 , pos - strSize.Height / 2), strSize, (Convert.ToInt32(pos / separat) * separat).ToString(), fontSize, Brushes.Black, StringAlignment.Far, StringAlignment.Center));
                    }
                    else
                    {
                        Add(CreateStringFigure(new(-strSize.Width                        , pos - strSize.Height / 2), strSize, (Convert.ToInt32(pos / separat) * separat).ToString(), fontSize, Brushes.Black, StringAlignment.Far, StringAlignment.Center));
                    }
                }
            }

            public CoordinateGridDraw() { }
        }
    }
}