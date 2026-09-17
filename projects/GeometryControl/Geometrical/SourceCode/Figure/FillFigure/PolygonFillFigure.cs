using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace Geometrical
{
    namespace Figure
    {
        public class PolygonFillFigure : BasicPalygonFigure
        {
            #region BasePalygonFigure
            protected override void Draw(Graphics g, IConvertTo? f = null)
            {
                var _Vertex = new PointF[Vertex.Length];
                for (int i = 0; i < _Vertex.Length; i++)
                {
                    _Vertex[i] = f is null ? Vertex[i] : f.ConvertToScale(Vertex[i]);
                }

                var _Polygon = new GraphicsPath();
                _Polygon.AddPolygon(_Vertex);
                g.FillPath(Color, _Polygon);
            }
            #endregion

            public PolygonFillFigure() { }
            public PolygonFillFigure(PointF[] v, PointF l, SizeF s, Brush c) : base(v, l, s, c) { }
        }
    }
}
