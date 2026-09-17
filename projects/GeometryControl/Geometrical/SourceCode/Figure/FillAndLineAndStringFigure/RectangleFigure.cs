using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace Geometrical
{
    namespace Figure
    {
        public class RectangleFigure : BasicTemplateFillAndLineAndStringFigure<RectangleFillFigure, RectangleLineFigure, StringFigure>
        {
            public RectangleFigure() { }
            public RectangleFigure(PointF l, SizeF s, Brush c, float ls, string t, float ts) : base(l, s, c, ls, t, ts) { }
        }
    }
}