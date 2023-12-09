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
            Figure1.Size = new(10, 20);
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

            var Figure3 = new CoordinateAxisDraw();
            Figure3.Location = new(50, 100);
            Figure3.Line.Color = Brushes.DarkCyan;
            Figure3.Fill.Color = Brushes.Gray;
            Figure3.Visible = true;
            geometricDrawing1.Add(Figure3);

            var Figure4 = new PolygonFigure();
            Figure4.Vertex = PolygonType.Hexagon(90f);
            Figure4.Location = new(150, 50);
            Figure4.Size = new(10, 10);
            Figure4.Line.LineSize = 3;
            Figure4.Line.Visible = false;
            Figure4.Fill.Color = Brushes.Olive;
            geometricDrawing1.Add(Figure4);

            var Figure5 = new PolygonFigure();
            Figure5.Vertex = PolygonType.Trigon(180f);
            Figure5.Location = new(200, 200);
            Figure5.Size = new(100, 100);
            Figure5.Fill.Color = Brushes.Olive;
            Figure5.Line.LineSize = 3;
            Figure5.Line.Visible = false;
            geometricDrawing1.Add(Figure5);
        }
    }
}