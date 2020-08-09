using System.Collections.Generic;
using System.Windows.Forms;
using WinFormsCtrlLibInputScreen.Base;

namespace CommandGenerator.Utility.InputDisplay.Strategy
{
	interface ILayout
	{
		List<Control> Layout(List<Control> src);
		List<Control> Layout(List<UserControlInput> src);
	}
}
