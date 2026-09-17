using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace Geometrical
{
    namespace Figure
    {
        public class PointFigure : BasicTemplateFillAndLineAndStringFigure<PointFillFigure, PointLineFigure, StringFigure>
        {
            public PointFigure() { }
            public PointFigure(PointF l, float s, Brush c, float ls, string t, float ts) : base(new(l.X - s, l.Y - s), new(s * 2, s * 2), c, ls, t, ts) { }
        }
    }
}