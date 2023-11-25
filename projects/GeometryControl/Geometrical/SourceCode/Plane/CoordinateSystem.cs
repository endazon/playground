using Geometrical.Figure;

namespace Geometrical
{
    namespace Plane
    {
        public enum CoordinateDirections
        {
            RightHanded = 0,
            LeftHanded  = 1,
        }
        public enum CoordinateRotates
        {
            Angle000 = 000,
            Angle090 = 090,
            Angle180 = 180,
            Angle270 = 270,
        }
        public interface ICoordinateSystem
        {
            PointF Origin { get; set; }
            CoordinateDirections Direction { get; set; }
            CoordinateRotates Rotation { get; set; }
            float ReducedScale { get; set; }
            float MagnificationRate { get; set; }
        }

        public class CoordinateSystem : ICoordinateSystem, IConvertTo, IConvertFrom
        {
            public PointF Origin { get; set; }
            public CoordinateDirections Direction { get; set; }
            public CoordinateRotates Rotation { get; set; }
            public float ReducedScale { get; set; }
            public float MagnificationRate { get; set; }

            #region IConvertTo
            public float ConvertToScale(float value)
            {
                return (value / ReducedScale) * MagnificationRate;
            }

            public PointF ConvertToScale(PointF value, PointF offset)
            {
                var direction = GetUnitDirection();
                return new(direction.X * ConvertToScale(value.X + offset.X), direction.Y * ConvertToScale(value.Y + offset.Y));
            }

            public PointF ConvertToScale(PointF value)
            {
                return ConvertToScale(value, Origin);
            }

            public SizeF ConvertToScale(SizeF value)
            {
                return new(ConvertToScale(value.Width), ConvertToScale(value.Height));
            }

            public RectangleF ConvertToScale(RectangleF value, PointF offset)
            {
                switch (Direction)
                {
                    case CoordinateDirections.RightHanded:
                        switch (Rotation)
                        {
                            case CoordinateRotates.Angle000:
                                return new(ConvertToScale(value.Location, offset), ConvertToScale(value.Size));
                            case CoordinateRotates.Angle090:
                                break;
                            case CoordinateRotates.Angle180:
                                break;
                            case CoordinateRotates.Angle270:
                                break;
                            default:
                                break;
                        }
                        break;
                    case CoordinateDirections.LeftHanded:
                        switch (Rotation)
                        {
                            case CoordinateRotates.Angle000:
                                break;
                            case CoordinateRotates.Angle090:
                                break;
                            case CoordinateRotates.Angle180:
                                return new(ConvertToScale(new PointF(value.Location.X, value.Location.Y + value.Height), offset), ConvertToScale(value.Size));
                            case CoordinateRotates.Angle270:
                                break;
                            default:
                                break;
                        }
                        break;
                    default:
                        break;
                }
                throw new Exception("CoordinateSystem Parameter Error");
            }

            public RectangleF ConvertToScale(RectangleF value)
            {
                return ConvertToScale(value, Origin);
            }

            public Pen ConvertToScale(Pen value)
            {
                return new(value.Color, ConvertToScale(value.Width));
            }

            public Font ConvertToScale(Font value)
            {
                return new(value.FontFamily, ConvertToScale(value.Size), value.Style, value.Unit);
            }

            public StringFormat ConvertToScale(StringFormat value)
            {
                var unit = GetUnitCoordinateSystem();
                var format = new StringFormat();
                if (unit.X == 0) { format.Alignment = value.Alignment; }
                else
                {
                    switch (value.Alignment)
                    {
                        case StringAlignment.Near:
                            format.Alignment = StringAlignment.Far;
                            break;
                        case StringAlignment.Far:
                            format.Alignment = StringAlignment.Near;
                            break;
                        case StringAlignment.Center:
                            format.Alignment = StringAlignment.Center;
                            break;
                        default:
                            format.Alignment = value.Alignment;
                            break;
                    }
                }
                if (unit.Y == 0) { format.LineAlignment = value.LineAlignment; }
                else
                {
                    switch (value.LineAlignment)
                    {
                        case StringAlignment.Near:
                            format.LineAlignment = StringAlignment.Far;
                            break;
                        case StringAlignment.Far:
                            format.LineAlignment = StringAlignment.Near;
                            break;
                        case StringAlignment.Center:
                            format.LineAlignment = StringAlignment.Center;
                            break;
                        default:
                            format.LineAlignment = value.LineAlignment;
                            break;
                    }
                }
                return format;
            }
            #endregion

            #region IConvertFrom
            public float ConvertFromScale(float value)
            {
                return (value * ReducedScale) / MagnificationRate;
            }

            public PointF ConvertFromScale(PointF value, PointF offset)
            {
                var direction = GetUnitDirection();
                return new(direction.X * ConvertFromScale(value.X - offset.X), direction.Y * ConvertFromScale(value.Y - offset.Y));
            }

            public PointF ConvertFromScale(PointF value)
            {
                return ConvertFromScale(value, ConvertToScale(Origin, default));
            }

            public SizeF ConvertFromScale(SizeF value)
            {
                return new(ConvertFromScale(value.Width), ConvertFromScale(value.Height));
            }

            public RectangleF ConvertFromScale(RectangleF value, PointF offset)
            {
                switch (Direction)
                {
                    case CoordinateDirections.RightHanded:
                        switch (Rotation)
                        {
                            case CoordinateRotates.Angle000:
                                return new(ConvertFromScale(value.Location, offset), ConvertFromScale(value.Size));
                            case CoordinateRotates.Angle090:
                                break;
                            case CoordinateRotates.Angle180:
                                break;
                            case CoordinateRotates.Angle270:
                                break;
                            default:
                                break;
                        }
                        break;
                    case CoordinateDirections.LeftHanded:
                        switch (Rotation)
                        {
                            case CoordinateRotates.Angle000:
                                break;
                            case CoordinateRotates.Angle090:
                                break;
                            case CoordinateRotates.Angle180:
                                return new(ConvertFromScale(new PointF(value.Location.X, value.Location.Y - value.Height), offset), ConvertFromScale(value.Size));
                            case CoordinateRotates.Angle270:
                                break;
                            default:
                                break;
                        }
                        break;
                    default:
                        break;
                }
                throw new Exception("CoordinateSystem Parameter Error");
            }

            public RectangleF ConvertFromScale(RectangleF value)
            {
                return ConvertFromScale(value, ConvertToScale(Origin, default));
            }

            public Pen ConvertFromScale(Pen value)
            {
                return new(value.Color, ConvertFromScale(value.Width));
            }

            public Font ConvertFromScale(Font value)
            {
                return new(value.FontFamily, ConvertFromScale(value.Size), value.Style, value.Unit);
            }

            public StringFormat ConvertFromScale(StringFormat value)
            {
                var unit = GetUnitCoordinateSystem();
                var format = new StringFormat();
                if (unit.X == 0) { format.Alignment     = value.Alignment     == StringAlignment.Far  ? StringAlignment.Near : value.Alignment    ; }
                else             { format.Alignment     = value.Alignment     == StringAlignment.Near ? StringAlignment.Far  : value.Alignment    ; }
                if (unit.Y == 0) { format.LineAlignment = value.LineAlignment == StringAlignment.Far  ? StringAlignment.Near : value.LineAlignment; }
                else             { format.LineAlignment = value.LineAlignment == StringAlignment.Near ? StringAlignment.Far  : value.LineAlignment; }
                return format;
            }
            #endregion

            public Point GetUnitCoordinateSystem()
            {
                switch (Direction)
                {
                    case CoordinateDirections.RightHanded:
                        switch (Rotation)
                        {
                            case CoordinateRotates.Angle000:
                                return new(0, 0);//右下
                            case CoordinateRotates.Angle090:
                                return new(1, 0);//右上
                            case CoordinateRotates.Angle180:
                                return new(1, 1);//右下
                            case CoordinateRotates.Angle270:
                                return new(0, 1);//左下
                            default:
                                break;
                        }
                        break;
                    case CoordinateDirections.LeftHanded:
                        switch (Rotation)
                        {
                            case CoordinateRotates.Angle000:
                                return new(1, 0);//右上
                            case CoordinateRotates.Angle090:
                                return new(0, 0);//左上
                            case CoordinateRotates.Angle180:
                                return new(0, 1);//左下
                            case CoordinateRotates.Angle270:
                                return new(0, 0);//右下
                            default:
                                break;
                        }
                        break;
                    default:
                        break;
                }
                throw new Exception("CoordinateSystem Parameter Error");
            }

            public Point GetUnitDirection()
            {
                var unit = GetUnitCoordinateSystem();
                return new Point(unit.X == 0 ? +1 : -1, unit.Y == 0 ? +1 : -1);
            }

            public void ChangeMagnificationRate(float rate)
            {
                if (MagnificationRate + rate <= 0) { return; }
                MagnificationRate += rate;
            }

            public void MovingOrigin(PointF offset)
            {
                var _offset = ConvertFromScale(offset, default);
                Origin = new(Origin.X + _offset.X, Origin.Y + _offset.Y);
            }

            public CoordinateSystem(PointF p, CoordinateDirections d, CoordinateRotates r, float s, float m)
            {
                Origin            = p;
                Direction         = d;
                Rotation          = r;
                ReducedScale      = s;
                MagnificationRate = m;
            }
            public CoordinateSystem() : this(
                new(),                           //原点(0,0)
                CoordinateDirections.RightHanded,//右手座標系
                CoordinateRotates.Angle000,      //0°回転
                1.0f,                            //1[mm]
                1.0f                             //100[%]
                )
            { }
        }


    }
}