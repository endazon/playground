using Geometrical.Figure;
using System.Drawing;

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
            public PointF ConvertToCoordinateDirection(PointF location)
            {
                switch (Direction)
                {
                    case CoordinateDirections.RightHanded:
                        switch (Rotation)
                        {
                            case CoordinateRotates.Angle000:
                                return new(+location.X, +location.Y);
                            case CoordinateRotates.Angle090:
                                return new(-location.Y, +location.X);
                            case CoordinateRotates.Angle180:
                                return new(-location.X, -location.Y);
                            case CoordinateRotates.Angle270:
                                return new(+location.Y, -location.X);
                            default:
                                break;
                        }
                        break;
                    case CoordinateDirections.LeftHanded:
                        switch (Rotation)
                        {
                            case CoordinateRotates.Angle000:
                                return new(-location.X, +location.Y);
                            case CoordinateRotates.Angle090:
                                return new(+location.Y, +location.X);
                            case CoordinateRotates.Angle180:
                                return new(+location.X, -location.Y);
                            case CoordinateRotates.Angle270:
                                return new(-location.Y, -location.X);
                            default:
                                break;
                        }
                        break;
                    default:
                        break;
                }
                throw new Exception("CoordinateSystem Parameter Error");
            }
            public SizeF ConvertToSizeDirection(SizeF size)
            {
                var convert = ConvertToCoordinateDirection(new(size.Width, size.Height));
                return new(Math.Abs(convert.X), Math.Abs(convert.Y));
            }
            public float ConvertToScale(float value)
            {
                return (value / ReducedScale) * MagnificationRate;
            }
            public PointF ConvertToScale(PointF value, PointF offset)
            {
                return ConvertToCoordinateDirection(new(ConvertToScale(value.X + offset.X), ConvertToScale(value.Y + offset.Y)));
            }

            public PointF ConvertToScale(PointF value)
            {
                return ConvertToScale(value, Origin);
            }

            public SizeF ConvertToScale(SizeF value)
            {
                return ConvertToSizeDirection(new(ConvertToScale(value.Width), ConvertToScale(value.Height)));
            }

            public RectangleF ConvertToScale(RectangleF value, PointF offset)
            {
                var location = ConvertToScale(value.Location, offset);
                var size = ConvertToScale(value.Size);
                switch (Direction)
                {
                    case CoordinateDirections.RightHanded:
                        switch (Rotation)
                        {
                            case CoordinateRotates.Angle000:
                                return new(location.X             , location.Y              , size.Width, size.Height);
                            case CoordinateRotates.Angle090:
                                return new(location.X - size.Width, location.Y              , size.Width, size.Height);
                            case CoordinateRotates.Angle180:
                                return new(location.X - size.Width, location.Y - size.Height, size.Width, size.Height);
                            case CoordinateRotates.Angle270:
                                return new(location.X             , location.Y - size.Height, size.Width, size.Height);
                            default:
                                break;
                        }
                        break;
                    case CoordinateDirections.LeftHanded:
                        switch (Rotation)
                        {
                            case CoordinateRotates.Angle000:
                                return new(location.X - size.Width, location.Y              , size.Width, size.Height);
                            case CoordinateRotates.Angle090:
                                return new(location.X             , location.Y              , size.Width, size.Height);
                            case CoordinateRotates.Angle180:
                                return new(location.X             , location.Y - size.Height, size.Width, size.Height);
                            case CoordinateRotates.Angle270:
                                return new(location.X - size.Width, location.Y - size.Height, size.Width, size.Height);
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
                var convert = ConvertToSizeDirection(new(Convert.ToInt32(value.Alignment), Convert.ToInt32(value.LineAlignment)));
                var format = (StringFormat)value.Clone();
                format.Alignment     = (StringAlignment)convert.Width;
                format.LineAlignment = (StringAlignment)convert.Height;
                if (unit.X != 0)
                {
                    switch (format.Alignment)
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
                            break;
                    }
                }
                if (unit.Y != 0)
                {
                    switch (format.LineAlignment)
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
                            break;
                    }
                }
                return format;
            }
            #endregion

            #region IConvertFrom
            public PointF ConvertFromCoordinateDirection(PointF location)
            {
                switch (Direction)
                {
                    case CoordinateDirections.RightHanded:
                        switch (Rotation)
                        {
                            case CoordinateRotates.Angle000:
                                return new(+location.X, +location.Y);
                            case CoordinateRotates.Angle090:
                                return new(+location.Y, -location.X);
                            case CoordinateRotates.Angle180:
                                return new(-location.X, -location.Y);
                            case CoordinateRotates.Angle270:
                                return new(-location.Y, +location.X);
                            default:
                                break;
                        }
                        break;
                    case CoordinateDirections.LeftHanded:
                        switch (Rotation)
                        {
                            case CoordinateRotates.Angle000:
                                return new(-location.X, +location.Y);
                            case CoordinateRotates.Angle090:
                                return new(+location.Y, +location.X);
                            case CoordinateRotates.Angle180:
                                return new(+location.X, -location.Y);
                            case CoordinateRotates.Angle270:
                                return new(-location.Y, -location.X);
                            default:
                                break;
                        }
                        break;
                    default:
                        break;
                }
                throw new Exception("CoordinateSystem Parameter Error");
            }
            public SizeF ConvertFromSizeDirection(SizeF size)
            {
                var convert = ConvertFromCoordinateDirection(new(size.Width, size.Height));
                return new(Math.Abs(convert.X), Math.Abs(convert.Y));
            }
            public float ConvertFromScale(float value)
            {
                return (value * ReducedScale) / MagnificationRate;
            }

            public PointF ConvertFromScale(PointF value, PointF offset)
            {
                var _value = ConvertFromCoordinateDirection(value);
                return new(ConvertFromScale(_value.X - offset.X), ConvertFromScale(_value.Y - offset.Y));
            }

            public PointF ConvertFromScale(PointF value)
            {
                return ConvertFromScale(value, new(ConvertToScale(Origin.X), ConvertToScale(Origin.Y)));
            }

            public SizeF ConvertFromScale(SizeF value)
            {
                return ConvertFromSizeDirection(new(ConvertFromScale(value.Width), ConvertFromScale(value.Height)));
            }

            public RectangleF ConvertFromScale(RectangleF value, PointF offset)
            {
                var location = ConvertFromScale(value.Location, offset);
                var size = ConvertFromScale(value.Size);
                switch (Direction)
                {
                    case CoordinateDirections.RightHanded:
                        switch (Rotation)
                        {
                            case CoordinateRotates.Angle000:
                                return new(location.X             , location.Y              , size.Width, size.Height);
                            case CoordinateRotates.Angle090:
                                return new(location.X             , location.Y - size.Height, size.Width, size.Height);
                            case CoordinateRotates.Angle180:
                                return new(location.X - size.Width, location.Y - size.Height, size.Width, size.Height);
                            case CoordinateRotates.Angle270:
                                return new(location.X             , location.Y - size.Height, size.Width, size.Height);
                            default:
                                break;
                        }
                        break;
                    case CoordinateDirections.LeftHanded:
                        switch (Rotation)
                        {
                            case CoordinateRotates.Angle000:
                                return new(location.X - size.Width, location.Y              , size.Width, size.Height);
                            case CoordinateRotates.Angle090:
                                return new(location.X             , location.Y              , size.Width, size.Height);
                            case CoordinateRotates.Angle180:
                                return new(location.X             , location.Y - size.Height, size.Width, size.Height);
                            case CoordinateRotates.Angle270:
                                return new(location.X - size.Width, location.Y - size.Height, size.Width, size.Height);
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
                return ConvertFromScale(value, new(ConvertToScale(Origin.X), ConvertToScale(Origin.Y)));
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
                var convert = ConvertFromSizeDirection(new(Convert.ToInt32(value.Alignment), Convert.ToInt32(value.LineAlignment)));
                var alignment = (StringAlignment)convert.Width;
                var lineAlignment = (StringAlignment)convert.Height;
                if (unit.X == 0) { value.Alignment = alignment; }
                else
                {
                    switch (alignment)
                    {
                        case StringAlignment.Near:
                            value.Alignment = StringAlignment.Far;
                            break;
                        case StringAlignment.Far:
                            value.Alignment = StringAlignment.Near;
                            break;
                        case StringAlignment.Center:
                            value.Alignment = StringAlignment.Center;
                            break;
                        default:
                            value.Alignment = alignment;
                            break;
                    }
                }
                if (unit.Y == 0) { value.LineAlignment = lineAlignment; }
                else
                {
                    switch (lineAlignment)
                    {
                        case StringAlignment.Near:
                            value.LineAlignment = StringAlignment.Far;
                            break;
                        case StringAlignment.Far:
                            value.LineAlignment = StringAlignment.Near;
                            break;
                        case StringAlignment.Center:
                            value.LineAlignment = StringAlignment.Center;
                            break;
                        default:
                            value.LineAlignment = lineAlignment;
                            break;
                    }
                }
                return value;
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
                                return new(0, 0);//左上
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
                                return new(1, 1);//右下
                            default:
                                break;
                        }
                        break;
                    default:
                        break;
                }
                throw new Exception("CoordinateSystem Parameter Error");
            }

            public void ChangeMagnificationRate(float rate)
            {
                if (MagnificationRate + rate <= 0) { return; }
                MagnificationRate += rate;
            }

            public void MovingOrigin(PointF offset)
            {
                Origin = new(Origin.X + offset.X, Origin.Y + offset.Y);
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