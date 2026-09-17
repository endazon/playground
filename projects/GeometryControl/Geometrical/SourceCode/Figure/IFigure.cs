using System.Drawing;

namespace Geometrical
{
    namespace Figure
    {
        #region IConvert
        public interface IConvertTo
        {
            public PointF ConvertToCoordinateDirection(PointF location);
            public SizeF ConvertToSizeDirection(SizeF size);
            float ConvertToScale(float value);
            PointF ConvertToScale(PointF value, PointF offset);
            PointF ConvertToScale(PointF value);
            SizeF ConvertToScale(SizeF value);
            RectangleF ConvertToScale(RectangleF value, PointF offset);
            RectangleF ConvertToScale(RectangleF value);
            Pen ConvertToScale(Pen value);
            Font ConvertToScale(Font value);
            StringFormat ConvertToScale(StringFormat value);
        }
        public interface IConvertFrom
        {
            public PointF ConvertFromCoordinateDirection(PointF location);
            public SizeF ConvertFromSizeDirection(SizeF size);
            float ConvertFromScale(float value);
            PointF ConvertFromScale(PointF value, PointF offset);
            PointF ConvertFromScale(PointF value);
            SizeF ConvertFromScale(SizeF value);
            RectangleF ConvertFromScale(RectangleF value, PointF offset);
            RectangleF ConvertFromScale(RectangleF value);
            Pen ConvertFromScale(Pen value);
            Font ConvertFromScale(Font value);
            StringFormat ConvertFromScale(StringFormat value);
        }
        #endregion

        #region IFigure
        public interface IFigure
        {
            bool Visible { get; set; }
            PointF Location { get; set; }
            SizeF Size { get; set; }
            RectangleF Rectangle { get; set; }
            Brush Color { get; set; }
            string Name { get; set; }
            object? Tag { get; set; }

            void Drawing(Graphics g, IConvertTo? f = null);
        }
        public interface ILineFigure : IFigure
        {
            float LineSize { get; set; }
            Pen Pen { get; set; }
        }
        public interface IStringFigure : IFigure
        {
            string Text { get; set; }
            float TextSize { get; set; }
            FontFamily FontFamily { get; set; }
            FontStyle Style { get; set; }
            GraphicsUnit Unit { get; set; }
            StringFormat Format { get; set; }
            Font Font { get; set; }
            bool AutoFontSizeAdjustment { get; set; }
        }
        #endregion

        #region IPolygonFigure
        public interface IPolygonFigure : IFigure
        {
            PointF[] Vertex { get; set; }
        }
        public interface IPolygonLineFigure : IPolygonFigure, ILineFigure { }
        #endregion

        #region TemplateFillAndLineAndStringFigure
        public interface ITemplateFillAndLineAndStringFigure<FillClass, LineClass, StringClass> : IFigure
        {
            FillClass Fill { get; set; }
            LineClass Line { get; set; }
            StringClass String { get; set; }
        }        
        #endregion
    }
}