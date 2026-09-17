using System.Runtime.InteropServices;
using System.Diagnostics;
using Geometrical.Figure;
using System.ComponentModel;
using System.Security.Cryptography.X509Certificates;
using Geometrical.Properties;

namespace Geometrical
{
    namespace Plane
    {
        #region Event
        public class SelectFigureChangedEventArgs : EventArgs
        {
            public SelectFigureChangedEventArgs(List<IFigure> figures)
            {
                SelectionItems = figures;
            }
            public List<IFigure> SelectionItems { get; }
        }
        public delegate void SelectFigureChangedEventHandler(object sender, SelectFigureChangedEventArgs e);

        public class MouseMouseMoveForPlaneEventArgs : MouseEventArgs
        {
            public MouseMouseMoveForPlaneEventArgs(MouseButtons button, int clicks, float x, float y, int delta)
                : base(button, clicks, Convert.ToInt32(x), Convert.ToInt32(x), delta)
            {
                XF = x;
                YF = y;
            }
            public float XF { get; }
            public float YF { get; }
            public PointF LocationF => new PointF(XF, YF);
        }
        public delegate void MouseMouseMoveForPlaneEventHandler(object sender, MouseMouseMoveForPlaneEventArgs e);
        #endregion

        public class FigurePlane : PictureBox, ICoordinateSystem
        {
            #region Win32 Constants
            private const int WH_KEYBOARD_LL = 0x000D;
            private const int WM_KEYDOWN = 0x0100;
            private const int WM_KEYUP = 0x0101;
            private const int WM_SYSKEYDOWN = 0x0104;
            private const int WM_SYSKEYUP = 0x0105;
            #endregion

            #region Win32API Structures
            [StructLayout(LayoutKind.Sequential)]
            private class KBDLLHOOKSTRUCT
            {
                public uint vkCode;
                public uint scanCode;
                public KBDLLHOOKSTRUCTFlags flags;
            }

            [Flags]
            private enum KBDLLHOOKSTRUCTFlags : uint
            {
                KEYEVENTF_EXTENDEDKEY = 0x0001,
                KEYEVENTF_KEYUP = 0x0002,
                KEYEVENTF_SCANCODE = 0x0003,
                KEYEVENTF_UNICODE = 0x0004,
            }
            #endregion

            #region Win32 Methods
            [DllImport("user32.dll", CharSet = CharSet.Auto, SetLastError = true)]
            private static extern IntPtr SetWindowsHookEx(int idHook, KeyboardProc lpfn, IntPtr hMod, uint dwThreadId);

            [DllImport("user32.dll", CharSet = CharSet.Auto, SetLastError = true)]
            [return: MarshalAs(UnmanagedType.Bool)]
            private static extern bool UnhookWindowsHookEx(IntPtr hhk);

            [DllImport("user32.dll", CharSet = CharSet.Auto, SetLastError = true)]
            private static extern IntPtr CallNextHookEx(IntPtr hhk, int nCode, IntPtr wParam, IntPtr lParam);

            [DllImport("kernel32.dll", CharSet = CharSet.Auto, SetLastError = true)]
            private static extern IntPtr GetModuleHandle(string lpModuleName);

            [DllImport("user32")]
            private static extern bool ReleaseCapture();
            #endregion

            #region Type
            private class KeyboardStatus
            {
                public List<Keys> DownKeyDataList { get; } = new();
                public bool Alt => DownKeyDataList.Contains(Keys.LMenu) || DownKeyDataList.Contains(Keys.RMenu);
                public bool Control => DownKeyDataList.Contains(Keys.LControlKey) || DownKeyDataList.Contains(Keys.RControlKey);
                public bool Shift => DownKeyDataList.Contains(Keys.LShiftKey) || DownKeyDataList.Contains(Keys.RShiftKey);

                public void OnKeyDown(KeyEventArgs ke)
                {
                    if (!DownKeyDataList.Contains(ke.KeyData))
                    {
                        DownKeyDataList.Add(ke.KeyData);
                    }
                }
                public void OnKeyUp(KeyEventArgs ke)
                {
                    if (DownKeyDataList.Contains(ke.KeyData))
                    {
                        DownKeyDataList.Remove(ke.KeyData);
                    }
                }
                public void Clear()
                {
                    DownKeyDataList.Clear();
                }
            }
            private class MouseStatus
            {
                public MouseButtons Button { get; private set; }
                public int Clicks { get; private set; }
                public Point Location { get; private set; }
                public Cursor FixedCursor { get; set; } = Cursors.Default;

                public void Init(Point pos)
                {
                    Button = MouseButtons.None;
                    Clicks = 0;
                    Location = pos;
                }
                public void Update(MouseEventArgs me)
                {
                    Button = me.Button;
                    Clicks = me.Clicks;
                    Location = me.Location;
                }
                public void Clear()
                {
                    Init(new());
                }
            }
            #endregion

            #region Delegate
            private delegate IntPtr KeyboardProc(int nCode, IntPtr wParam, IntPtr lParam);
            #endregion

            #region Readonly
            private readonly KeyboardStatus keyboardStatus = new();
            private readonly MouseStatus mouseStatus = new();
            private readonly Cursor CursorRotat = Cursors.No/*new Cursor(Resources.RotationArrows.GetHicon())*/;
            #endregion

            #region Fields
            private KeyboardProc? proc = null;
            private IntPtr hookId = IntPtr.Zero;
            private int z1KeyEventParam = WM_SYSKEYUP;
            private PointF axisOfRotation = new();
            #endregion

            #region Hook
            private void HookKeyboard()
            {
                using (Process curProcess = Process.GetCurrentProcess())
                using (ProcessModule? curModule = curProcess.MainModule)
                {
                    if (curModule == null) { return; }
                    if (curModule.ModuleName == null) { return; }
                    if (hookId != IntPtr.Zero) { return; }

                    //GC対策
                    proc = HookProcedure;

                    //フックを行う
                    //第１引数：フックするイベントの種類
                    //        　１３はキーボードフックを表す
                    //第２引数：フック時のメソッドのアドレス
                    //　　　　　フックメソッドを登録する
                    //第３引数：インスタンスハンドル
                    //        　現在実行中のハンドルを渡す
                    //第４引数：スレッドID
                    //　　　　　０を指定すると、すべてのスレッドでフックされる
                    hookId = SetWindowsHookEx(
                        /* int idHook        = */WH_KEYBOARD_LL,
                        /* KeyboardProc lpfn = */proc,
                        /* IntPtr hMod       = */GetModuleHandle(curModule.ModuleName),
                        /* uint dwThreadId   = */0
                    );
                }
            }
            private IntPtr HookProcedure(int nCode, IntPtr wParam, IntPtr lParam)
            {
                if (nCode >= 0)
                {
                    var kb = (KBDLLHOOKSTRUCT?)Marshal.PtrToStructure(lParam, typeof(KBDLLHOOKSTRUCT));
                    if (kb != null)
                    {
                        var keyEventParam = (int)wParam;
                        switch (z1KeyEventParam)
                        {
                            case WM_KEYDOWN:
                            case WM_SYSKEYDOWN:
                                switch (keyEventParam)
                                {
                                    case WM_KEYUP:
                                    case WM_SYSKEYUP:
                                        OnKeyUp(new KeyEventArgs((Keys)kb.vkCode));
                                        break;
                                }
                                break;
                            case WM_KEYUP:
                            case WM_SYSKEYUP:
                                switch (keyEventParam)
                                {
                                    case WM_KEYDOWN:
                                    case WM_SYSKEYDOWN:
                                        OnKeyDown(new KeyEventArgs((Keys)kb.vkCode));
                                        break;
                                }
                                break;
                        }
                        z1KeyEventParam = keyEventParam;
                    }
                }
                return CallNextHookEx(hookId, nCode, wParam, lParam);
            }
            private void UnHookKeyboard()
            {
                UnhookWindowsHookEx(hookId);
                hookId = IntPtr.Zero;
            }
            #endregion

            #region EventHandler
            [Browsable(true)]
            [Localizable(true)]
            [Category("Geometrical.Plane")]
            [Description("図面選択イベント")]
            public event SelectFigureChangedEventHandler? SelectFigureChanged = null;

            [Browsable(true)]
            [Localizable(true)]
            [Category("Geometrical.Plane")]
            [Description("平面上のマウス移動イベント")]
            public event MouseMouseMoveForPlaneEventHandler? MouseMouseMoveForPlane = null;
            #endregion

            #region Properties
            [Browsable(false)]
            protected CoordinateSystem System { get; set; } = new();

            [Browsable(true)]
            [Localizable(true)]
            [Category("CoordinateSystem")]
            [Description("2次元平面上の原点")]
            [TypeConverter(typeof(PointFConverter))]
            public PointF Origin
            {
                get => System.Origin;
                set
                {
                    System.Origin = value;
                }
            }

            [Browsable(true)]
            [Localizable(true)]
            [Category("CoordinateSystem")]
            [Description("2次元平面上の座標系")]
            [DefaultValue(typeof(CoordinateDirections), "RightHanded")]
            public CoordinateDirections Direction
            {
                get => System.Direction;
                set
                {
                    System.Direction = value;
                }
            }

            [Browsable(true)]
            [Localizable(true)]
            [Category("CoordinateSystem")]
            [Description("2次元平面上の向き")]
            [DefaultValue(typeof(CoordinateRotates), "Angle000")]
            public CoordinateRotates Rotation
            {
                get => System.Rotation;
                set
                {
                    System.Rotation = value;
                }
            }

            [Browsable(true)]
            [Localizable(true)]
            [Category("CoordinateSystem")]
            [Description("2次元平面上の縮尺")]
            [DefaultValue(1.0f)]
            public float ReducedScale
            {
                get => System.ReducedScale;
                set
                {
                    System.ReducedScale = value;
                }
            }

            [Browsable(true)]
            [Localizable(true)]
            [Category("CoordinateSystem")]
            [Description("2次元平面上の拡大率")]
            [DefaultValue(1.0f)]
            public float MagnificationRate
            {
                get => System.MagnificationRate;
                set
                {
                    System.MagnificationRate = value;
                }
            }

            [Browsable(false)]
            public FigureList FigureList { get; } = new();

            [Browsable(false)]
            public FigureList SelectionItems { get; } = new();
            [Browsable(false)]
            public FigureList SelectionLineItems { get; } = new();

            [Browsable(true)]
            [Localizable(true)]
            [Category("FigurePlane")]
            [Description("編集禁止")]
            [DefaultValue(false)]
            public bool EditingProhibited { get; set; } = false;
            #endregion

            #region Func
            public Cursor GetSelectFigureCursor(PointF location)
            {
                if (EditingProhibited)
                {
                    var AreaSize = System.ConvertFromScale(5);
                    var TopRightArea = new RectangleF(
                        SelectionItems.Rectangle.Right - AreaSize,
                        SelectionItems.Rectangle.Top,
                        AreaSize,
                        AreaSize
                        );
                    var BottomRightArea = new RectangleF(
                        SelectionItems.Rectangle.Right  - AreaSize,
                        SelectionItems.Rectangle.Bottom - AreaSize,
                        AreaSize,
                        AreaSize
                        );
                    var RightArea = new RectangleF(
                        SelectionItems.Rectangle.Right - AreaSize,
                        SelectionItems.Rectangle.Top,
                        AreaSize,
                        SelectionItems.Rectangle.Height
                        );
                    var BottomArea = new RectangleF(
                        SelectionItems.Rectangle.Left,
                        SelectionItems.Rectangle.Bottom - AreaSize,
                        SelectionItems.Rectangle.Width,
                        AreaSize
                        );

                    if (0 < SelectionItems.Count)
                    {
                        if (TopRightArea.Contains(location))
                        {
                            axisOfRotation = location;
                            return CursorRotat;
                        }
                        else if (BottomRightArea.Contains(location))
                        {
                            return Cursors.SizeNWSE;
                        }
                        else if(RightArea.Contains(location))
                        {
                            return Cursors.SizeWE;
                        }
                        else if (BottomArea.Contains(location))
                        {
                            return Cursors.SizeNS;
                        }
                        else if (SelectionItems.Rectangle.Contains(location))
                        {
                            return Cursors.SizeAll;
                        }
                    }
                }
                return Cursors.Default;
            }
            #endregion

            #region Interface
            public void SelectedFigureAdd(IFigure? figure)
            {
                if (figure != null)
                {
                    void SetSelectFigure(IFigure selectFigure, IFigure origin)
                    {
                        selectFigure.Location = origin.Location;
                        selectFigure.Size     = origin.Size;
                        selectFigure.Color    = Brushes.Cyan;
                        SelectionItems.Add(origin);
                        SelectionLineItems.Add(selectFigure);
                    }
                    if (FigureOperation.IsTypeMatchRectangleFigure(figure))
                    {
                        var selectFigure      = new RectangleLineFigure();
                        var origin            = FigureOperation.CastRectangleFigure(figure);
                        selectFigure.LineSize = origin.Line.LineSize;
                        SetSelectFigure(selectFigure, origin);
                    }
                    else if (FigureOperation.IsTypeMatchEllipseFigure(figure))
                    {
                        var selectFigure      = new EllipseLineFigure();
                        var origin            = FigureOperation.CastEllipseFigure(figure);
                        selectFigure.LineSize = origin.Line.LineSize;
                        SetSelectFigure(selectFigure, origin);

                    }
                    else if (FigureOperation.IsTypeMatchPolygonFigure(figure))
                    {
                        var selectFigure      = new PolygonLineFigure();
                        var origin            = FigureOperation.CastPolygonFigure(figure);
                        selectFigure.Vertex   = origin.Vertex;
                        selectFigure.LineSize = origin.Line.LineSize;
                        SetSelectFigure(selectFigure, origin);
                    }
                    if(2 > SelectionLineItems.Count)
                    {
                        SelectionLineItems.Line.Visible = false;
                    }
                    else
                    {
                        SelectionLineItems.Line.Color = Brushes.DarkCyan;
                        SelectionLineItems.Line.LineSize = 1;
                        SelectionLineItems.Line.Visible = true;
                    }

                    OnSelectFigureChanged(new SelectFigureChangedEventArgs(new List<IFigure>(SelectionItems)));
                }
                else
                {
                    OnSelectFigureChanged(new SelectFigureChangedEventArgs(new()));
                }
                Refresh();
            }
            public void SelectedFigureClear()
            {
                SelectionItems.Clear();
                SelectionLineItems.Clear();
                Refresh();
            }
            #endregion

            #region OnEventDefinition
            protected virtual void OnPreDrawing(PaintEventArgs pe)
            {
                var unit = System.GetUnitCoordinateSystem();
                pe.Graphics.TranslateTransform(unit.X * Width, unit.Y * Height);
            }
            protected virtual void OnDrawing(PaintEventArgs pe)
            {
                FigureList.Drawing(pe.Graphics, System);
                SelectionLineItems.Drawing(pe.Graphics, System);
            }
            protected virtual void OnPostDrawing(PaintEventArgs pe)
            {
                pe.Graphics.ResetTransform();
            }
            protected virtual void OnMouseMouseMoveForPlane(MouseMouseMoveForPlaneEventArgs me)
            {
                MouseMouseMoveForPlane?.Invoke(this, me);
            }
            protected virtual void OnSelectFigureChanged(SelectFigureChangedEventArgs e)
            {
                SelectFigureChanged?.Invoke(this, e);
            }
            #endregion

            #region OnEvent
            protected override void OnBackColorChanged(EventArgs e)
            {
                FigureList.Color = new SolidBrush(BackColor);
                base.OnBackColorChanged(e);
            }
            protected override void OnKeyDown(KeyEventArgs ke)
            {
                keyboardStatus.OnKeyDown(ke);
                base.OnKeyDown(ke);
            }
            protected override void OnKeyUp(KeyEventArgs ke)
            {
                keyboardStatus.OnKeyUp(ke);
                base.OnKeyUp(ke);
            }
            protected override void OnPaint(PaintEventArgs pe)
            {
                OnPreDrawing(pe);
                OnDrawing(pe);
                OnPostDrawing(pe);
                base.OnPaint(pe);
            }
            protected override void OnMouseEnter(EventArgs e)
            {
                HookKeyboard(); 
                mouseStatus.Init(Cursor.Position);
                base.OnMouseEnter(e);
            }
            protected override void OnMouseWheel(MouseEventArgs me)
            {
                if (keyboardStatus.Control)
                {
                    //ホイールの回転回数が、
                    if (me.Delta > 0)
                    {
                        //正の場合、拡大
                        System.ChangeMagnificationRate(+1.0f);
                    }
                    else
                    {
                        //負の場合、縮小
                        System.ChangeMagnificationRate(-1.0f);
                    }
                    Refresh();
                }
                mouseStatus.Update(me);
                base.OnMouseWheel(me);
            }
            protected override void OnMouseDown(MouseEventArgs me)
            {
                mouseStatus.FixedCursor = Cursors.Default;
                if (me.Button == MouseButtons.Left)
                {
                    //図形選択
                    var unit = System.GetUnitCoordinateSystem();
                    if (!keyboardStatus.Control) { SelectedFigureClear(); }
                    SelectedFigureAdd(FigureList.SelectFigure(System, new PointF(me.X - unit.X * Width, me.Y - unit.Y * Height), SelectionItems));

                    var location = System.ConvertFromScale(new PointF(me.X - unit.X * Width, me.Y - unit.Y * Height));
                    mouseStatus.FixedCursor = GetSelectFigureCursor(location);
                }
                Refresh();
                mouseStatus.Update(me);
                base.OnMouseDown(me);
            }
            protected override void OnMouseMove(MouseEventArgs me)
            {
                var unit        = System.GetUnitCoordinateSystem();
                var location    = System.ConvertFromScale(new PointF(me.X                   - unit.X * Width, me.Y                   - unit.Y * Height));
                var z1location  = System.ConvertFromScale(new PointF(mouseStatus.Location.X - unit.X * Width, mouseStatus.Location.Y - unit.Y * Height));
                var variation   = new PointF(location.X - z1location.X, location.Y - z1location.Y);
                var MinimumSize = System.ReducedScale;

                if (ClientRectangle.Contains(me.Location))
                {
                    if (mouseStatus.Button == MouseButtons.Right)
                    {
                        //左クリックされていたら、原点移動
                        System.MovingOrigin(variation);
                        Refresh();
                    }

                    if(mouseStatus.FixedCursor != Cursors.Default)
                    {

                        if (mouseStatus.FixedCursor == CursorRotat)
                        {
                            var rotatPoint = new PointF(location.X - axisOfRotation.X, location.Y - axisOfRotation.Y);
                            var radian = Math.Atan2(+rotatPoint.Y, -rotatPoint.X);
                            var angle = (radian * 180 / Math.PI) + 180;

                            //TODO:FugureRotate
                            Debug.WriteLine(angle);
                        }
                        else if (mouseStatus.FixedCursor == Cursors.SizeAll)
                        {
                            SelectionItems.Location = new(
                                SelectionItems.Location.X + variation.X,
                                SelectionItems.Location.Y + variation.Y
                                );
                        }
                        else if (mouseStatus.FixedCursor == Cursors.SizeNWSE)
                        {
                            SelectionItems.Size = new(
                                MinimumSize < SelectionItems.Size.Width  + variation.X ? SelectionItems.Size.Width  + variation.X : MinimumSize,
                                MinimumSize < SelectionItems.Size.Height + variation.Y ? SelectionItems.Size.Height + variation.Y : MinimumSize
                                );
                        }
                        else if (mouseStatus.FixedCursor == Cursors.SizeWE)
                        {
                            SelectionItems.Size = new(
                                MinimumSize < SelectionItems.Size.Width + variation.X ? SelectionItems.Size.Width + variation.X : MinimumSize,
                                SelectionItems.Size.Height
                                );
                        }
                        else if (mouseStatus.FixedCursor == Cursors.SizeNS)
                        {
                            SelectionItems.Size = new(
                                SelectionItems.Size.Width,
                                MinimumSize < SelectionItems.Size.Height + variation.Y ? SelectionItems.Size.Height + variation.Y : MinimumSize
                                );
                        }

                        SelectionLineItems.Location = SelectionItems.Location;
                        SelectionLineItems.Size = SelectionItems.Size;
                        Refresh();
                    }
                    else
                    {
                        Cursor = GetSelectFigureCursor(location);
                    }
                }
                else
                {
                    ReleaseCapture();
                }

#if false//デバッグ用
                {
                    var a = System.ConvertToScale(new PointF(10, 20));
                    var b = System.ConvertFromScale(a);
                    var c = System.ConvertToScale(new SizeF(10, 20));
                    var d = System.ConvertFromScale(c);
                    var e = System.ConvertToScale(new RectangleF(10, 20, 10, 20));
                    var f = System.ConvertFromScale(e);
                }
#endif
                OnMouseMouseMoveForPlane(new MouseMouseMoveForPlaneEventArgs(me.Button, me.Clicks, location.X, location.Y, me.Delta));
                mouseStatus.Update(me);
                base.OnMouseMove(me);
            }
            protected override void OnMouseUp(MouseEventArgs me)
            {
                mouseStatus.FixedCursor = Cursors.Default;
                mouseStatus.Update(me);
                base.OnMouseUp(me);
            }
            protected override void OnMouseLeave(EventArgs e)
            {
                UnHookKeyboard();
                keyboardStatus.Clear();
                mouseStatus.FixedCursor = Cursors.Default;
                mouseStatus.Clear();
                base.OnMouseLeave(e);
            }
            #endregion

            #region Construct
            public FigurePlane() { }
            #endregion

            #region Destruct
            ~FigurePlane()
            {
                UnHookKeyboard();
            }
            #endregion
        }
    }
}