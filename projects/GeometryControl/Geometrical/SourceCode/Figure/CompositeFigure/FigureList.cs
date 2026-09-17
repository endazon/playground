using Geometrical.Plane;
using System.Collections;

namespace Geometrical
{
    namespace Figure
    {
        public class FigureList : RectangleFigure, IList<IFigure>
        {
            private List<IFigure> Items { get; } = new();

            public FigureList() 
            {
                Fill.Visible = false;
                Line.Visible = false;
                String.Visible = false;
            }

            private void UpdateItems()
            {
                float L = 0, T = 0, R = 0, B = 0;
                for (int i = 0; i < Count; i++)
                {
                    var rectangle = Items[i].Rectangle;
                    if (i == 0)
                    {
                        L = rectangle.Left  ;
                        T = rectangle.Top   ;
                        R = rectangle.Right ;
                        B = rectangle.Bottom;
                    }
                    else
                    {
                        L = rectangle.Left   < L ? rectangle.Left   : L;
                        T = rectangle.Top    < T ? rectangle.Top    : T;
                        R = rectangle.Right  > R ? rectangle.Right  : R;
                        B = rectangle.Bottom > B ? rectangle.Bottom : B;
                    }
                }
                base.Location = new(             L ,              T );
                base.Size     = new(Math.Abs(R - L), Math.Abs(B - T));
            }

            public IFigure? SelectFigure(CoordinateSystem system, PointF point, FigureList? ignore = null)
            {
                foreach (var item in this.Reverse())
                {
                    if (ignore != null)
                    {
                        if (ignore.Contains(item)) { continue; }
                    }
                    if (item.Rectangle.Contains(system.ConvertFromScale(point)))
                    {
                        return item;
                    }
                }
                return null;
            }

            #region BasicFigure
            public override bool Visible
            {
                get => base.Visible;
                set
                {
                    base.Visible = value;
                    foreach (var item in Items)
                    {
                        item.Visible = Visible;
                    }
                }
            }
            public override PointF Location
            {
                get => base.Location;
                set
                {
                    var offset = new PointF(value.X - base.Location.X, value.Y - base.Location.Y);
                    foreach (var item in Items)
                    {
                        item.Location = new(
                            /* X = */item.Location.X + offset.X,
                            /* Y = */item.Location.Y + offset.Y
                            );
                    }
                    base.Location = value;
                }
            }
            public override SizeF Size
            {
                get => base.Size;
                set
                {
                    var deformationRate = new SizeF(value.Width / base.Size.Width, value.Height / base.Size.Height);
                    if (deformationRate.Width != float.NaN && deformationRate.Height != float.NaN)
                    {
                        foreach (var item in Items)
                        {
                            item.Location = new(
                                /* X = */Location.X + ((item.Location.X - Location.X) * deformationRate.Width ),
                                /* Y = */Location.Y + ((item.Location.Y - Location.Y) * deformationRate.Height)
                                );
                            item.Size = new(
                                /* Width  = */item.Size.Width  * deformationRate.Width,
                                /* Height = */item.Size.Height * deformationRate.Height
                                );
                        }
                    }
                    base.Size = value;
                }
            }
            protected override void Draw(Graphics g, IConvertTo? f = null)
            {
                Fill.Drawing(g, f);
                foreach (var item in Items)
                {
                    item.Drawing(g, f);
                }
                Line.Drawing(g, f);
                String.Drawing(g, f);
            }
            #endregion

            #region IList<IFigure>
            public IFigure this[int index] { get => Items[index]; set => Items[index] = value; }

            public int Count => Items.Count;

            public bool IsReadOnly => false;

            public void Add(IFigure item)
            {
                Items.Add(item);
                UpdateItems();
            }

            public void Clear()
            {
                Items.Clear();
                UpdateItems();
            }

            public bool Contains(IFigure item) => Items.Contains(item);

            public void CopyTo(IFigure[] array, int arrayIndex) => Items.CopyTo(array, arrayIndex);

            public IEnumerator<IFigure> GetEnumerator() => Items.GetEnumerator();

            public int IndexOf(IFigure item) => Items.IndexOf(item);

            public void Insert(int index, IFigure item)
            {
                Items.Insert(index, item);
                UpdateItems();
            }

            public bool Remove(IFigure item)
            {
                var ret = Items.Remove(item);
                UpdateItems();
                return ret;
            }

            public void RemoveAt(int index)
            {
                Items.RemoveAt(index);
                UpdateItems();
            }

            IEnumerator IEnumerable.GetEnumerator() => Items.GetEnumerator();
            #endregion
        }
    }
}