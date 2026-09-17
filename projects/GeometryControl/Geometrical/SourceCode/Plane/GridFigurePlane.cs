using Geometrical.Figure;
using System.ComponentModel;

namespace Geometrical
{
    namespace Plane
    {
        public class GridFigurePlane : FigurePlane
        {

            #region Properties
            [Browsable(false)]
            private CoordinateAxisDraw CoordinateAxisDrawer { get; } = new();

            [Browsable(false)]
            private CoordinateGridDraw CoordinateGridDrawer { get; } = new();
            #endregion

            #region OnEvent
            protected override void OnDrawing(PaintEventArgs pe)
            {
                CoordinateGridDrawer.UpdateGrid(Size, System, 5);
                CoordinateGridDrawer.Drawing(pe.Graphics, System);
                base.OnDrawing(pe);
                CoordinateAxisDrawer.Drawing(pe.Graphics, new CoordinateSystem(new(10, 10), System.Direction, System.Rotation, 1.0f, 1.0f));
            }
            #endregion
        }
    }
}