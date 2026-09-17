using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace Geometrical
{
    namespace Figure
    {
        public class CircleFigure : BasicTemplateFillAndLineAndStringFigure<CircleFillFigure, CircleLineFigure, StringFigure>
        {
            public CircleFigure() { }
            public CircleFigure(PointF l, float s, Brush c, float ls, string t, float ts) : base(l, new(s, s), c, ls, t, ts) { }
        }
    }
}