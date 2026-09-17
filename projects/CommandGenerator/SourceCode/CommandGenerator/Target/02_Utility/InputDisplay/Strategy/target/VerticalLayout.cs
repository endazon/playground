using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;
using WinFormsCtrlLibInputScreen.Base;

namespace CommandGenerator.Utility.InputDisplay.Strategy
{
	class VerticalLayout : ILayout
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
			var ret = _Layout(src);
			ret     = _Layout(ret);
			return ret;
		}

		private List<Control> _Layout(List<Control> src)
		{
			var offset = 0;
			foreach(var item in src)
			{
				item.CreateControl();
				item.Location = new Point(X , Y + offset);
				offset += item.Height;
			}

			return src;
		}

		private List<Control> _Layout(List<UserControlInput> src)
		{
			var max_size = 0;
			foreach (var item in src)
			{
				max_size = max_size < item.LabelWidth() ? item.LabelWidth() : max_size;
			}

			max_size += 10;
			foreach (var item in src)
			{
				var span = max_size - item.LabelWidth();
				item.LineUp((uint)span);
			}
			
			return src.Select(e => (Control)e).ToList();
		}

	}
}
