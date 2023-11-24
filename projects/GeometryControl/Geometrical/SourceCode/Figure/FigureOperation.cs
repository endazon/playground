
namespace Geometrical
{
    namespace Figure
    {
        public static class FigureOperation
        {
            #region RectangleFigure
            public static bool IsTypeMatchRectangleFillFigure(Type t) => typeof(RectangleFillFigure) == t;
            public static bool IsTypeMatchRectangleLineFigure(Type t) => typeof(RectangleLineFigure) == t;
            public static bool IsTypeMatchRectangleeFillOrLineFigure(Type t) => IsTypeMatchRectangleFillFigure(t) || IsTypeMatchRectangleLineFigure(t);
            public static bool IsTypeMatchSquareFillFigure(Type t) => typeof(SquareFillFigure) == t;
            public static bool IsTypeMatchSquareLineFigure(Type t) => typeof(SquareLineFigure) == t;
            public static bool IsTypeMatchSquareFillOrLineFigure(Type t) => IsTypeMatchSquareFillFigure(t) || IsTypeMatchSquareLineFigure(t);
            #endregion

            #region EllipseFigure
            public static bool IsTypeMatchEllipseFillFigure(Type t) => typeof(EllipseFillFigure) == t;
            public static bool IsTypeMatchEllipseLineFigure(Type t) => typeof(EllipseLineFigure) == t;
            public static bool IsTypeMatchEllipseFillOrLineFigure(Type t) => IsTypeMatchEllipseFillFigure(t) || IsTypeMatchEllipseLineFigure(t);
            public static bool IsTypeMatchCircleFillFigure(Type t) => typeof(CircleFillFigure) == t;
            public static bool IsTypeMatchCircleLineFigure(Type t) => typeof(CircleLineFigure) == t;
            public static bool IsTypeMatchCircleFillOrLineFigure(Type t) => IsTypeMatchCircleFillFigure(t) || IsTypeMatchCircleLineFigure(t);
            public static bool IsTypeMatchPointFillFigure(Type t) => typeof(PointFillFigure) == t;
            public static bool IsTypeMatchPointLineFigure(Type t) => typeof(PointLineFigure) == t;
            public static bool IsTypeMatchPointFillOrLineFigure(Type t) => IsTypeMatchPointFillFigure(t) || IsTypeMatchPointLineFigure(t);
            #endregion

            #region CompositeFigure
            public static bool IsTypeMatchRectangleFigure(Type t) => typeof(RectangleFigure) == t;
            public static RectangleFigure CastRectangleFigure(object obj) => (RectangleFigure)obj;

            public static bool IsTypeMatchEllipseFigure(Type t) => typeof(EllipseFigure) == t;
            public static EllipseFigure CastEllipseFigure(object obj) => (EllipseFigure)obj;
            #endregion

            public static bool IsTypeMatchAnyRectangle(Type t) => IsTypeMatchRectangleeFillOrLineFigure(t) || IsTypeMatchSquareFillOrLineFigure(t) || IsTypeMatchRectangleFigure(t);
            public static bool IsTypeMatchAnyEllipse(Type t) => IsTypeMatchEllipseFillOrLineFigure(t) || IsTypeMatchCircleFillOrLineFigure(t) || IsTypeMatchPointFillOrLineFigure(t) || IsTypeMatchEllipseFigure(t);
            public static bool IsTypeMatchAny(Type t) => IsTypeMatchAnyRectangle(t) || IsTypeMatchAnyEllipse(t);
        }
    }
}