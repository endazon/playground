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

        public class CoordinateSystem : IConvertTo, IConvertFrom
        {
            public PointF Origin { get; set; }
            public CoordinateDirections Direction { get; set; }
            public CoordinateRotates Rotation { get; set; }
            public float Scale { get; set; }
            public float MagnificationRate { get; set; }

            #region IConvertTo
            public float ConvertToScale(float value)
            {
                return (value / Scale) * MagnificationRate;
            }

            public PointF ConvertToScale(PointF value, PointF offset)
            {
                return new (ConvertToScale(value.X + offset.X), ConvertToScale(value.Y + offset.Y));
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
                return new(ConvertToScale(value.Location, offset), ConvertToScale(value.Size));
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
            #endregion

            #region IConvertFrom
            public float ConvertFromScale(float value)
            {
                return (value * Scale) / MagnificationRate;
            }

            public PointF ConvertFromScale(PointF value, PointF offset)
            {
                return new(ConvertFromScale(value.X) - offset.X, ConvertFromScale(value.Y) - offset.Y);
            }

            public PointF ConvertFromScale(PointF value)
            {
                return ConvertFromScale(value, Origin);
            }

            public SizeF ConvertFromScale(SizeF value)
            {
                return new(ConvertFromScale(value.Width), ConvertFromScale(value.Height));
            }

            public RectangleF ConvertFromScale(RectangleF value, PointF offset)
            {
                return new(ConvertFromScale(value.Location, offset), ConvertFromScale(value.Size));
            }

            public RectangleF ConvertFromScale(RectangleF value)
            {
                return ConvertFromScale(value, Origin);
            }

            public Pen ConvertFromScale(Pen value)
            {
                return new(value.Color, ConvertFromScale(value.Width));
            }

            public Font ConvertFromScale(Font value)
            {
                return new(value.FontFamily, ConvertFromScale(value.Size), value.Style, value.Unit);
            }
            #endregion

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
                Scale             = s;
                MagnificationRate = m;
            }
            public CoordinateSystem() : this(
                new(),                           //Œ´“_(0,0)
                CoordinateDirections.RightHanded,//‰EŽèÀ•WŒn
                CoordinateRotates.Angle000,      //0‹‰ñ“]
                1.0f,                            //1[mm]
                1.0f                             //100[%]
                )
            { }
        }


    }
}