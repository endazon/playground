using Geometrical;
using Geometrical.Figure;
using Geometrical.Plane;

namespace DebuggingForms
{
    public partial class DebuggingForm : Form
    {
        public DebuggingForm()
        {
            InitializeComponent();

            var System = new CoordinateSystem();
            //System.Scale = 1000;
            System.MagnificationRate = 5;
            geometricDrawing1.System = System;

            var Figure = new RectangleFillFigure();
            Figure.Location = new(50, 50);
            Figure.Size = new(10, 10);
            Figure.Color = Brushes.Black;
            geometricDrawing1.Add(Figure);
        }
    }
}