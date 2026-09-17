using System.Drawing;
using System.Windows.Forms;
using CommandGenerator.Target.Common.Range;
using WinFormsCtrlLibInputScreen.Base;

namespace CommandGenerator.Utility.InputDisplay.Adapter
{
	interface IInput
	{
		#region PpublicMethod
		IInput SetFont(Font font);
		IInput SetName(string text);
		IInput SetValue(string value);
		IInput SetValue(string[] value);
		IInput SetValues(string value);
		IInput SetRange(Range<int> range);
		UserControlInput GetInputDisplay();
		#endregion
	}
}
