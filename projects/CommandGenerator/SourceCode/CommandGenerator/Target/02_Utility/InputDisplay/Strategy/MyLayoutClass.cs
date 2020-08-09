using System;
using System.Collections.Generic;
using System.Windows.Forms;
using WinFormsCtrlLibInputScreen.Base;

namespace CommandGenerator.Utility.InputDisplay.Strategy
{
	class MyLayoutClass
	{
		private readonly ILayout layout;
		private readonly Panel panel;
		private List<Control> contents;

		public MyLayoutClass(ILayout layout)
		{
			this.layout = layout;
		}
		public MyLayoutClass(ILayout layout, Panel panel) : this(layout)
		{
			this.panel = panel;
		}

		private void Panel_Layout(object sender, LayoutEventArgs e)
		{
			var panel = (Panel)sender;

			panel.SuspendLayout();

			panel.Controls.Clear();
			panel.Controls.AddRange(contents.ToArray());

			panel.ResumeLayout(false);
			panel.PerformLayout();
		}

		public void Layout(List<Control> src)
		{
			contents = layout.Layout(src);
		}

		public void Layout(List<UserControlInput> src)
		{
			contents = layout.Layout(src);
		}

		public void Update()
		{
			Update(panel);
		}

		public void Update(Panel panel)
		{
			panel.Layout += new LayoutEventHandler(Panel_Layout);
			panel.PerformLayout();
			panel.Layout -= new LayoutEventHandler(Panel_Layout);
		}

		public Control Search(string name)
		{
			return contents.Find(item => item.Name == name);
		}

		/// <summary>
		/// 詳細コピーメソッド
		/// </summary>
		/// <returns>コピーデータ</returns>
		public MyLayoutClass Clone()
		{
			return (MyLayoutClass)MemberwiseClone();
		}
	}
}

