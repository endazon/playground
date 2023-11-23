namespace Geometrical
{
    namespace Figure
    {
        public class StraightLineFigure : BasicFigure, ILineFigure
        {
            public override  PointF Location
            {
                get => base.Location; 
                set
                {
                    var z1Location = base.Location;
                    base.Location = value;

                    _StartPoint = new(
                        /* X = */_StartPoint.X + (base.Location.X - z1Location.X),
                        /* Y = */_StartPoint.Y + (base.Location.Y - z1Location.Y)
                        );

                    _EndPoint = new(
                        /* X = */_EndPoint.X + (base.Location.X - z1Location.X),
                        /* Y = */_EndPoint.Y + (base.Location.Y - z1Location.Y)
                        );
                }
            }
            public override SizeF Size
            {
                get => base.Size;
                set
                {
                    base.Size = value;

                    _StartPoint = new(
                        /* X = */base.Location.X == _StartPoint.X ? base.Location.X : base.Location.X + base.Size.Width,
                        /* Y = */base.Location.Y == _StartPoint.Y ? base.Location.Y : base.Location.Y + base.Size.Height
                        );

                    _EndPoint = new(
                        /* X = */base.Location.X == _EndPoint.X ? base.Location.X : base.Location.X + base.Size.Width,
                        /* Y = */base.Location.Y == _EndPoint.Y ? base.Location.Y : base.Location.Y + base.Size.Height
                        );
                }
            }

            public float LineSize { get; set; } = 1.0f;
            public Pen Pen
            {
                get => new(Color, LineSize);
                set
                {
                    Color    = value.Brush;
                    LineSize = value.Width;
                }
            }      

            private PointF _StartPoint = new();
            public virtual PointF StartPoint
            {
                get => _StartPoint;
                set
                {
                    _StartPoint = value;

                    base.Location = new(
                        /* Width  = */_StartPoint.X < _EndPoint.X ? _StartPoint.X : _EndPoint.X,
                        /* Height = */_StartPoint.Y < _EndPoint.Y ? _StartPoint.Y : _EndPoint.Y
                        );
                    base.Size = new(
                        /* Width  = */Math.Abs(_EndPoint.X - _StartPoint.X),
                        /* Height = */Math.Abs(_EndPoint.Y - _StartPoint.Y)
                        );
                }
            }
            private PointF _EndPoint = new();
            public virtual PointF EndPoint
            {
                get => _EndPoint;
                set
                {
                    _EndPoint = value;

                    base.Location = new(
                        /* Width  = */_StartPoint.X < _EndPoint.X ? _StartPoint.X : _EndPoint.X,
                        /* Height = */_StartPoint.Y < _EndPoint.Y ? _StartPoint.Y : _EndPoint.Y
                        );
                    base.Size = new(
                        /* Width  = */Math.Abs(_EndPoint.X - _StartPoint.X),
                        /* Height = */Math.Abs(_EndPoint.Y - _StartPoint.Y)
                        );
                }
            }

            protected override void Draw(Graphics g, IConvertTo? f = null)
            {
                var _Pen        = f is null ? Pen        : f.ConvertToScale(Pen       );
                var _StartPoint = f is null ? StartPoint : f.ConvertToScale(StartPoint);
                var _EndPoint   = f is null ? EndPoint   : f.ConvertToScale(EndPoint  );
                g.DrawLine(_Pen, _StartPoint, _EndPoint);
            }
        }
    }
}