using Geometrical.Figure;
using Geometrical.Plane;
using System.Collections;
using System.ComponentModel;

namespace Geometrical
{
    public partial class GeometricDrawing : UserControl, IList<IBasicFigure>
    {
        #region EventHandler
        public event Plane.SelectFigureChangedEvent SelectFigureChanged
        {
            add    => Plane1.SelectFigureChanged += value;
            remove => Plane1.SelectFigureChanged -= value;
        }
        #endregion

        #region Properties
        public CoordinateSystem System { get => Plane1.System; set => Plane1.System = value; }
        #endregion

        #region IList<IBasicFigure>
        public IBasicFigure this[int index] { get => Plane1.FigureList[index]; set => Plane1.SelectionItems[index] = value; }

        public int Count => Plane1.FigureList.Count;

        public bool IsReadOnly => Plane1.FigureList.IsReadOnly;

        public void Add(IBasicFigure item) => Plane1.FigureList.Add(item);

        public void Clear() => Plane1.FigureList.Clear();

        public bool Contains(IBasicFigure item) => Plane1.FigureList.Contains(item);

        public void CopyTo(IBasicFigure[] array, int arrayIndex) => Plane1.FigureList.CopyTo(array, arrayIndex);

        public IEnumerator<IBasicFigure> GetEnumerator() => Plane1.FigureList.GetEnumerator();

        public int IndexOf(IBasicFigure item) => Plane1.FigureList.IndexOf(item);

        public void Insert(int index, IBasicFigure item) => Plane1.FigureList.Insert(index, item);

        public bool Remove(IBasicFigure item) => Plane1.FigureList.Remove(item);

        public void RemoveAt(int index) => Plane1.FigureList.RemoveAt(index);

        IEnumerator IEnumerable.GetEnumerator() => Plane1.FigureList.GetEnumerator();
        #endregion

        public GeometricDrawing()
        {
            InitializeComponent();

            splitContainer1.SplitterDistance = splitContainer1.Height - statusStrip1.Height;
            splitContainer1.IsSplitterFixed = true;
        }
    }
}