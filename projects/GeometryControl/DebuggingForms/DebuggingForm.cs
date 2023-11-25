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

            var Figure1 = new RectangleFigure();
            Figure1.Location = new(50, 50);
            Figure1.Size = new(10, 10);
            Figure1.Line.Color = Brushes.Orange;
            Figure1.Fill.Color = Brushes.Red;
            Figure1.String.Text = "aaa";
            Figure1.String.Color = Brushes.Black;
            geometricDrawing1.Add(Figure1);

            var Figure2 = new EllipseFigure();
            Figure2.Location = new(100, 50);
            Figure2.Size = new(10, 10);
            Figure2.Line.Color = Brushes.Lime;
            Figure2.Fill.Color = Brushes.Green;
            Figure2.String.Text = "bbb";
            Figure2.String.Color = Brushes.Black;
            geometricDrawing1.Add(Figure2);

            geometricDrawing1.Direction = CoordinateDirections.LeftHanded;
            geometricDrawing1.Rotation = CoordinateRotates.Angle180;
        }
    }
}