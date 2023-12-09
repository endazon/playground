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
        public static class PolygonType
        {
            public static PointF[] PolygonVertexCalculation(uint vertexnum, float angle = 0, float radius = 0.5f)
            {
                var vertex = new PointF[vertexnum];
                for (int i = 0; i < vertex.Length; i++)
                {
                    var vertexDegress = ((360f / vertex.Length) * i) + angle;
                    var vertexRadian  = (float)Math.PI * vertexDegress / 180f;
                    vertex[i] = new PointF(
                        (float)Math.Sin(vertexRadian) * radius - radius,
                        (float)Math.Cos(vertexRadian) * radius - radius
                        );
                }
                return vertex;
            }
            public static PointF[] Monogon     (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(1      , angle, radius);//一角形   Monogon      モノゴン
            public static PointF[] Henagon     (float angle = 0, float radius = 0.5f) => Monogon                 (         angle, radius);//一角形   Henagon      ヘナゴン
            public static PointF[] Digon       (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(2      , angle, radius);//二角形   Digon        ディゴン
            public static PointF[] Trigon      (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(3      , angle, radius);//三角形   Trigon       トリゴン
            public static PointF[] Tetragon    (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(4      , angle, radius);//四角形   Tetragon     テトラゴン
            public static PointF[] Pentagon    (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(5      , angle, radius);//五角形   Pentagon     ペンタゴン
            public static PointF[] Hexagon     (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(6      , angle, radius);//六角形   Hexagon      ヘキサゴン
            public static PointF[] Heptagon    (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(7      , angle, radius);//七角形   Heptagon     ヘプタゴン
            public static PointF[] Septagon    (float angle = 0, float radius = 0.5f) => Heptagon                (         angle, radius);//七角形   Septagon     セプタゴン
            public static PointF[] Octagon     (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(8      , angle, radius);//八角形   Octagon      オクタゴン
            public static PointF[] Enneagon    (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(9      , angle, radius);//九角形   Enneagon     エニアゴン
            public static PointF[] Nonagon     (float angle = 0, float radius = 0.5f) => Enneagon                (         angle, radius);//九角形   Nonagon      ノナゴン
            public static PointF[] Decagon     (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(10     , angle, radius);//十角形   Decagon      デカゴン
            public static PointF[] Undecagon   (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(11     , angle, radius);//十一角形 Undecagon    アンデカゴン
            public static PointF[] Hendecagon  (float angle = 0, float radius = 0.5f) => Undecagon               (         angle, radius);//十一角形 Hendecagon   ヘンデカゴン
            public static PointF[] Dodecagon   (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(12     , angle, radius);//十二角形 Dodecagon    ドデカゴン
            public static PointF[] Tridecagon  (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(13     , angle, radius);//十三角形 Tridecagon   トリデカゴン
            public static PointF[] Tetradecagon(float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(14     , angle, radius);//十四角形 Tetradecagon テトラデカゴン
            public static PointF[] Pentadecagon(float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(15     , angle, radius);//十五角形 Pentadecagon ペンタデカゴン
            public static PointF[] Hexadecagon (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(16     , angle, radius);//十六角形 Hexadecagon  ヘクサデカゴン
            public static PointF[] Heptadecagon(float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(17     , angle, radius);//十七角形 Heptadecagon ヘプタデカゴン
            public static PointF[] Octadecagon (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(18     , angle, radius);//十八角形 Octadecagon  オクタデカゴン
            public static PointF[] Enneadecagon(float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(19     , angle, radius);//十九角形 Enneadecagon エネアデカゴン
            public static PointF[] Nonadecagon (float angle = 0, float radius = 0.5f) => Enneadecagon            (         angle, radius);//十九角形 Nonadecagon  ノナデカゴン
            public static PointF[] Icosagon    (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(20     , angle, radius);//二十角形 Icosagon     イコサゴン
            public static PointF[] Triacontagon(float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(30     , angle, radius);//三十角形 Triacontagon トリアコンタゴン
            public static PointF[] Hectogon    (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(100    , angle, radius);//百角形   Hectogon     ヘクトゴン
            public static PointF[] Chiliagon   (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(1000   , angle, radius);//千角形   Chiliagon    チリアゴン
            public static PointF[] Myriagon    (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(10000  , angle, radius);//一万角形 Myriagon     ミリアゴン
            public static PointF[] Megagon     (float angle = 0, float radius = 0.5f) => PolygonVertexCalculation(1000000, angle, radius);//百万角形 Megagon      メガゴン
        }

        public class PolygonFillFigure : BasePalygonFigure
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

        public class PolygonLineFigure : BasePalygonFigure, IPolygonLineFigure
        {
            #region ILineFigure
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
            #endregion

            #region BasePalygonFigure
            protected override void Draw(Graphics g, IConvertTo? f = null)
            {
                var _Pen = f is null ? Pen : f.ConvertToScale(Pen);
                var _Vertex = new PointF[Vertex.Length];
                for (int i = 0; i < _Vertex.Length; i++)
                {
                    _Vertex[i] = f is null ? Vertex[i] : f.ConvertToScale(Vertex[i]);
                }

                var _Polygon = new GraphicsPath();
                _Polygon.AddPolygon(_Vertex);
                g.DrawPath(Pen, _Polygon);
            }
            #endregion

            public PolygonLineFigure() { }
            public PolygonLineFigure(PointF[] v, PointF l, SizeF s, Brush c, float ls) : base(v, l, s, c) { LineSize = ls; }
        }
    }
}
