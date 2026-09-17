
namespace Geometrical
{
    namespace Figure
    {
        public static class FigureOperation
        {
            #region RectangleFigure
            public static bool IsTypeMatchRectangleFillFigure(object obj) => obj is RectangleFillFigure;
            public static bool IsTypeMatchRectangleLineFigure(object obj) => obj is RectangleLineFigure;
            public static bool IsTypeMatchRectangleeFillOrLineFigure(object obj) => IsTypeMatchRectangleFillFigure(obj) || IsTypeMatchRectangleLineFigure(obj);
            #endregion

            #region EllipseFigure
            public static bool IsTypeMatchEllipseFillFigure(object obj) => obj is EllipseFillFigure;
            public static bool IsTypeMatchEllipseLineFigure(object obj) => obj is EllipseLineFigure;
            public static bool IsTypeMatchEllipseFillOrLineFigure(object obj) => IsTypeMatchEllipseFillFigure(obj) || IsTypeMatchEllipseLineFigure(obj);
            #endregion

            #region CompositeFigure
            public static bool IsTypeMatchRectangleFigure(object obj) => obj is RectangleFigure;
            public static RectangleFigure CastRectangleFigure(object obj) => (RectangleFigure)obj;

            public static bool IsTypeMatchEllipseFigure(object obj) => obj is EllipseFigure;
            public static EllipseFigure CastEllipseFigure(object obj) => (EllipseFigure)obj;

            public static bool IsTypeMatchPolygonFigure(object obj) => obj is PolygonFigure;
            public static PolygonFigure CastPolygonFigure(object obj) => (PolygonFigure)obj;
            #endregion

            public static bool IsTypeMatchAnyRectangle(object obj) => IsTypeMatchRectangleeFillOrLineFigure(obj) || IsTypeMatchRectangleFigure(obj);
            public static bool IsTypeMatchAnyEllipse(object obj) => IsTypeMatchEllipseFillOrLineFigure(obj) || IsTypeMatchEllipseFigure(obj);
            public static bool IsTypeMatchAny(object obj) => IsTypeMatchAnyRectangle(obj) || IsTypeMatchAnyEllipse(obj) || IsTypeMatchPolygonFigure(obj);
        }
    }
}