using System.Collections.Generic;
using System.Linq;
using System.Windows.Forms;
using WinFormsCtrlLibInputScreen.Base;
using System.Drawing;
using System;

namespace CommandGenerator.Utility.InputDisplay.Strategy
{
	class HorizontalityLayout : ILayout
	{
		readonly int X = 5;
		readonly int Y = 15;

		public List<Control> Layout(List<Control> src)
		{
			var ret = _Layout(src);
			return ret;
		}

		public List<Control> Layout(List<UserControlInput> src)
		{
			var ret = _Layout(src.Select(e => (Control)e).ToList());
			return ret;
		}

		private List<Control> _Layout(List<Control> src)
		{
			var offset = 0;
			foreach (var item in src)
			{
				item.CreateControl();
				item.Location = new Point(X + offset, Y);
				offset += item.Width;
			}

			return src;
		}
	}
}
