using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace Geometrical
{
    namespace Figure
    {
        public class PolygonFigure : BasicTemplatePolygonFillAndLineAndStringFigure<PolygonFillFigure, PolygonLineFigure, StringFigure>
        {
            public PolygonFigure() { }
            public PolygonFigure(PointF[] v, PointF l, SizeF s, Brush c, float ls, string t, float ts) : base(v, l, s, c, ls, t, ts) { }
        }
    }
}