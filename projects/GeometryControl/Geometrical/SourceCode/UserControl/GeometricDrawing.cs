using Geometrical.Figure;
using Geometrical.Plane;
using System.Collections;
using System.ComponentModel;
using System.Diagnostics;
using System.Windows.Forms;

namespace Geometrical
{
    public partial class GeometricDrawing : UserControl, ICoordinateSystem, IList<IFigure>
    {
        #region EventHandler
        [Browsable(true)]
        [Localizable(true)]
        [Category("Geometrical.Plane")]
        [Description("図面選択イベント")]
        public event SelectFigureChangedEventHandler SelectFigureChanged
        {
            add => Plane1.SelectFigureChanged += value;
            remove => Plane1.SelectFigureChanged -= value;
        }

        [Browsable(true)]
        [Localizable(true)]
        [Category("Geometrical.Plane")]
        [Description("平面上のマウス移動イベント")]
        public event MouseMouseMoveForPlaneEventHandler MouseMouseMoveForPlane
        {
            add => Plane1.MouseMouseMoveForPlane += value;
            remove => Plane1.MouseMouseMoveForPlane -= value;
        }
        #endregion

        #region ICoordinateSystem
        [Browsable(true)]
        [Localizable(true)]
        [Category("CoordinateSystem")]
        [Description("2次元平面上の原点")]
        [TypeConverter(typeof(PointFConverter))]
        public PointF Origin
        {
            get => Plane1.Origin;
            set
            {
                Plane1.Origin = value;
            }
        }

        [Browsable(true)]
        [Localizable(true)]
        [Category("CoordinateSystem")]
        [Description("2次元平面上の座標系")]
        [DefaultValue(typeof(CoordinateDirections), "RightHanded")]
        public CoordinateDirections Direction
        {
            get => Plane1.Direction;
            set
            {
                Plane1.Direction = value;
            }
        }

        [Browsable(true)]
        [Localizable(true)]
        [Category("CoordinateSystem")]
        [Description("2次元平面上の向き")]
        [DefaultValue(typeof(CoordinateRotates), "Angle000")]
        public CoordinateRotates Rotation
        {
            get => Plane1.Rotation;
            set
            {
                Plane1.Rotation = value;
            }
        }

        [Browsable(true)]
        [Localizable(true)]
        [Category("CoordinateSystem")]
        [Description("2次元平面上の縮尺")]
        [DefaultValue(1.0f)]
        public float ReducedScale
        {
            get => Plane1.ReducedScale;
            set
            {
                Plane1.ReducedScale = value;
            }
        }

        [Browsable(true)]
        [Localizable(true)]
        [Category("CoordinateSystem")]
        [Description("2次元平面上の拡大率")]
        [DefaultValue(1.0f)]
        public float MagnificationRate
        {
            get => Plane1.MagnificationRate;
            set
            {
                Plane1.MagnificationRate = value;
            }
        }
        #endregion

        #region IList<IFigure>
        public IFigure this[int index] { get => Plane1.FigureList[index]; set => Plane1.SelectionItems[index] = value; }

        [Browsable(false)]
        public int Count => Plane1.FigureList.Count;

        [Browsable(false)]
        public bool IsReadOnly => Plane1.FigureList.IsReadOnly;

        public void Add(IFigure item) => Plane1.FigureList.Add(item);

        public void Clear() => Plane1.FigureList.Clear();

        public bool Contains(IFigure item) => Plane1.FigureList.Contains(item);

        public void CopyTo(IFigure[] array, int arrayIndex) => Plane1.FigureList.CopyTo(array, arrayIndex);

        public IEnumerator<IFigure> GetEnumerator() => Plane1.FigureList.GetEnumerator();

        public int IndexOf(IFigure item) => Plane1.FigureList.IndexOf(item);

        public void Insert(int index, IFigure item) => Plane1.FigureList.Insert(index, item);

        public bool Remove(IFigure item) => Plane1.FigureList.Remove(item);

        public void RemoveAt(int index) => Plane1.FigureList.RemoveAt(index);

        IEnumerator IEnumerable.GetEnumerator() => Plane1.FigureList.GetEnumerator();
        #endregion

        [Browsable(true)]
        [Localizable(true)]
        [Category("FigurePlane")]
        [Description("編集禁止")]
        [DefaultValue(false)]
        public bool EditingProhibited
        {
            get => Plane1.EditingProhibited;
            set
            {
                Plane1.EditingProhibited = value;
            }
        }

        public GeometricDrawing()
        {
            InitializeComponent();
        }

        protected override void OnResize(EventArgs e)
        {
            base.OnResize(e);
        }

        private void Plane1_Paint(object sender, PaintEventArgs e)
        {
            var s = sender as FigurePlane;

            toolStripStatusLabel2.Text = $"Scale:{s.ReducedScale}[mm]";
            toolStripStatusLabel3.Text = $"Rate:{s.MagnificationRate * 100}[%]";
        }

        private void Plane1_MouseMouseMoveForPlane(object sender, MouseMouseMoveForPlaneEventArgs me)
        {
            toolStripStatusLabel1.Text = $"Location:X={me.XF}[mm] ,Y={me.YF}[mm]";
        }
    }
}