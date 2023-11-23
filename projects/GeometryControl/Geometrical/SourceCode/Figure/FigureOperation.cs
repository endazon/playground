
namespace Geometrical
{
    namespace Figure
    {

        using CompositeRectangleFigure = TmpCompositeFigure<RectangleFillFigure, RectangleLineFigure, StringFigure>;
        using CompositeEllipseFigure = TmpCompositeFigure<EllipseFillFigure, EllipseLineFigure, StringFigure>;
        public static class FigureOperation
        {
            #region RectangleFigure
            public static bool IsTypeMatchRectangleFillFigure(Type t) => typeof(RectangleFillFigure) == t;
            public static bool IsTypeMatchRectangleLineFigure(Type t) => typeof(RectangleLineFigure) == t;
            public static bool IsTypeMatchRectangleFigure(Type t) => IsTypeMatchRectangleFillFigure(t) || IsTypeMatchRectangleLineFigure(t);
            public static bool IsTypeMatchSquareFillFigure(Type t) => typeof(SquareFillFigure) == t;
            public static bool IsTypeMatchSquareLineFigure(Type t) => typeof(SquareLineFigure) == t;
            public static bool IsTypeMatchSquareFigure(Type t) => IsTypeMatchRectangleFigure(t) || IsTypeMatchSquareLineFigure(t);
            #endregion

            #region EllipseFigure
            public static bool IsTypeMatchEllipseFillFigure(Type t) => typeof(EllipseFillFigure) == t;
            public static bool IsTypeMatchEllipseLineFigure(Type t) => typeof(EllipseLineFigure) == t;
            public static bool IsTypeMatchEllipseFigure(Type t) => IsTypeMatchEllipseFillFigure(t) || IsTypeMatchEllipseLineFigure(t);
            public static bool IsTypeMatchCircleFillFigure(Type t) => typeof(CircleFillFigure) == t;
            public static bool IsTypeMatchCircleLineFigure(Type t) => typeof(CircleLineFigure) == t;
            public static bool IsTypeMatchCircleFigure(Type t) => IsTypeMatchCircleFillFigure(t) || IsTypeMatchCircleLineFigure(t);
            public static bool IsTypeMatchPointFillFigure(Type t) => typeof(PointFillFigure) == t;
            public static bool IsTypeMatchPointLineFigure(Type t) => typeof(PointLineFigure) == t;
            public static bool IsTypeMatchPointFigure(Type t) => IsTypeMatchPointFillFigure(t) || IsTypeMatchPointLineFigure(t);
            #endregion

            #region TmpCompositeFigure
            public static bool IsTypeMatchCompositeRectangleFigure(Type t) => typeof(CompositeRectangleFigure) == t;
            public static CompositeRectangleFigure CreateCompositeRectangleFigure() => new();
            public static CompositeRectangleFigure CreateCompositeRectangleFigure(PointF l, SizeF s, Brush c, float ls, string t, float ts) => new(l, s, c, ls, t, ts);
            public static CompositeRectangleFigure CastCompositeRectangleFigure(object obj) => (CompositeRectangleFigure)obj;

            public static bool IsTypeMatchCompositeEllipseFigure(Type t) => typeof(CompositeEllipseFigure) == t;
            public static CompositeEllipseFigure CreateCompositeEllipseFigure() => new();
            public static CompositeEllipseFigure CreateCompositeEllipseFigure(PointF l, SizeF s, Brush c, float ls, string t, float ts) => new(l, s, c, ls, t, ts);
            public static CompositeEllipseFigure CastCompositeEllipseFigure(object obj) => (CompositeEllipseFigure)obj;
            public static bool IsTypeMatchCompositeFigure(Type t) => IsTypeMatchCompositeRectangleFigure(t) || IsTypeMatchCompositeEllipseFigure(t);
            #endregion

            public static bool IsTypeMatchRectangle(Type t) => IsTypeMatchSquareFillFigure(t) || IsTypeMatchSquareFigure(t) || IsTypeMatchCompositeRectangleFigure(t);
            public static bool IsTypeMatchEllipse(Type t) => IsTypeMatchEllipseFigure(t) || IsTypeMatchCircleFillFigure(t) || IsTypeMatchPointLineFigure(t) || IsTypeMatchCompositeEllipseFigure(t);
            public static bool IsTypeMatch(Type t) => IsTypeMatchRectangle(t) || IsTypeMatchEllipse(t);
        }
    }
}