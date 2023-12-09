using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace Geometrical
{
    namespace Figure
    {
        public class EllipseFigure : BasicTemplateFillAndLineAndStringFigure<EllipseFillFigure, EllipseLineFigure, StringFigure>
        {
            public EllipseFigure() { }
            public EllipseFigure(PointF l, SizeF s, Brush c, float ls, string t, float ts) : base(l, s, c, ls, t, ts) { }
        }
    }
}