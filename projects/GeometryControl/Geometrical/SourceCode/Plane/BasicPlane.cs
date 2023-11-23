using System.Runtime.InteropServices;
using System.Diagnostics;
using Geometrical.Figure;
using System.ComponentModel;

namespace Geometrical
{
    namespace Plane
    {
        #region Event
        public class SelectFigureChangedEventArgs : EventArgs
        {
            public SelectFigureChangedEventArgs(List<IBasicFigure> figures)
            {
                SelectionItems = figures;
            }
            public List<IBasicFigure> SelectionItems { get; }
        }
        public delegate void SelectFigureChangedEvent(object sender, SelectFigureChangedEventArgs e);
        #endregion

        public class BasicPlane : PictureBox
        {
            #region Win32 Constants
            private const int WH_KEYBOARD_LL = 0x000D;
            private const int WM_KEYDOWN     = 0x0100;
            private const int WM_KEYUP       = 0x0101;
            private const int WM_SYSKEYDOWN  = 0x0104;
            private const int WM_SYSKEYUP    = 0x0105;
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
                KEYEVENTF_KEYUP       = 0x0002,
                KEYEVENTF_SCANCODE    = 0x0003,
                KEYEVENTF_UNICODE     = 0x0004,
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
                public List<Keys> DownKeyDataList { get; private set; } = new();
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
            }
            private class MouseStatus
            {
                public MouseButtons Button { get; private set; }
                public int Clicks { get; private set; }
                public Point Location { get; private set; }

                public void Update(MouseEventArgs me)
                {
                    Button   = me.Button;
                    Clicks   = me.Clicks;
                    Location = me.Location;
                }
                public void Clear()
                {
                    Button   = MouseButtons.None;
                    Clicks   = 0;
                    Location = new();
                }
            }
            #endregion

            #region Delegate
            private delegate IntPtr KeyboardProc(int nCode, IntPtr wParam, IntPtr lParam);
            #endregion

            #region Readonly
            private readonly KeyboardStatus keyboardStatus = new();
            private readonly MouseStatus    mouseStatus    = new();
            #endregion

            #region Fields
            private KeyboardProc? proc = null;
            private IntPtr hookId      = IntPtr.Zero;
            #endregion

            #region Hook
            private void HookKeyboard()
            {
                using (Process curProcess = Process.GetCurrentProcess())
                using (ProcessModule? curModule = curProcess.MainModule)
                {
                    if (curModule            == null)       { return; }
                    if (curModule.ModuleName == null)       { return; }
                    if (hookId               != IntPtr.Zero){ return; }

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
                        switch ((int)wParam)
                        {
                            case WM_KEYDOWN:
                            case WM_SYSKEYDOWN:
                                OnKeyDown(new KeyEventArgs((Keys)kb.vkCode));
                                break;
                            case WM_KEYUP:
                            case WM_SYSKEYUP:
                                OnKeyUp(new KeyEventArgs((Keys)kb.vkCode));
                                break;
                        }
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
            public event SelectFigureChangedEvent? SelectFigureChanged = null;
            #endregion

            #region Properties
            public CoordinateSystem System { get; set; } = new();
            public FigureList FigureList { get; } = new();
            public List<IBasicFigure> SelectionItems { get; } = new();
            #endregion

            #region OnEvent
            protected override void OnBackColorChanged(EventArgs e)
            {
                FigureList.Color = new SolidBrush(BackColor);
                base.OnBackColorChanged(e);
            }
            protected override void OnResize(EventArgs e)
            {
                //グリッド更新
                base.OnResize(e);
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
                FigureList.Drawing(pe.Graphics, System);
                base.OnPaint(pe);
            }
            protected override void OnMouseEnter(EventArgs e)
            {
                HookKeyboard();
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
                base.OnMouseWheel(me);
            }
            protected override void OnMouseDown(MouseEventArgs me)
            {
                if (me.Button == MouseButtons.Left)
                {
                    //図形選択
                    var figure = FigureList.SelectFigure(System, me.Location, SelectionItems);
                    if (figure != null)
                    {
                        if (!keyboardStatus.Control)
                        {
                            foreach (var item in SelectionItems)
                            {
                                FigureList.Remove(item);
                            }
                            SelectionItems.Clear();
                        }

                        ILineFigure selectFigure = new RectangleLineFigure();
                        var type = figure.GetType();
                        if (FigureOperation.IsTypeMatchEllipse(type))
                        {
                            selectFigure = new EllipseLineFigure();
                        }
                        if (FigureOperation.IsTypeMatchCompositeRectangleFigure(type))
                        {
                            selectFigure.LineSize = FigureOperation.CastCompositeRectangleFigure(figure).Line.LineSize;
                        }
                        else if (FigureOperation.IsTypeMatchCompositeEllipseFigure(type))
                        {
                            selectFigure.LineSize = FigureOperation.CastCompositeEllipseFigure(figure).Line.LineSize;
                        }
                        selectFigure.Location = figure.Location;
                        selectFigure.Size = figure.Size;
                        selectFigure.Color = Brushes.Cyan;
                        selectFigure.Tag = figure;
                        SelectionItems.Add(selectFigure);
                        FigureList.Add(selectFigure);

                        if(SelectFigureChanged != null)
                        {
                            var list = new List<IBasicFigure>();
                            foreach (var item in SelectionItems)
                            {
                                if(item.Tag == null) { continue; }
                                list.Add((IBasicFigure)item.Tag);
                            }
                            SelectFigureChanged(this, new SelectFigureChangedEventArgs(list));
                        }
                    }
                    else
                    {
                        foreach (var item in SelectionItems)
                        {
                            FigureList.Remove(item);
                        }

                        if (SelectFigureChanged != null)
                        {
                            SelectFigureChanged(this, new SelectFigureChangedEventArgs(new()));
                        }
                    }
                }
                Refresh();
                mouseStatus.Update(me);
                base.OnMouseDown(me);
            }
            protected override void OnMouseMove(MouseEventArgs me)
            {
                if (ClientRectangle.Contains(me.Location))
                {
                    if (mouseStatus.Button == MouseButtons.Right)
                    {
                        //左クリックされていたら、原点移動
                        System.MovingOrigin(new(me.X - mouseStatus.Location.X, me.Y - mouseStatus.Location.Y));
                        Refresh();
                    }
                    mouseStatus.Update(me);
                }
                else
                {
                    ReleaseCapture();
                }
                base.OnMouseMove(me);
            }
            protected override void OnMouseUp(MouseEventArgs me)
            {
                mouseStatus.Update(me);
                base.OnMouseUp(me);
            }
            protected override void OnMouseLeave(EventArgs e)
            {
                UnHookKeyboard();
                mouseStatus.Clear();
                base.OnMouseLeave(e);
            }
            #endregion

            #region Construct
            public BasicPlane() { }
            #endregion

            #region Destruct
            ~BasicPlane()
            {
                UnHookKeyboard();
            }
            #endregion
        }
    }
}