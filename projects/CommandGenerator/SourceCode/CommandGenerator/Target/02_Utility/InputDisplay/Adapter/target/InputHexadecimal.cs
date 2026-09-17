using CommandGenerator.Target.Common.Range;
using System;
using System.Drawing;
using System.Windows.Forms;
using WinFormsCtrlLibInputScreen.Base;

namespace CommandGenerator.Utility.InputDisplay.Adapter
{
	class InputHexadecimal : IInput
	{
		private readonly WinFormsCtrlLibInputScreen.InputHexadecimal input;

		#region Constructor
		public InputHexadecimal()
		{
			input = new WinFormsCtrlLibInputScreen.InputHexadecimal();
		}
		public InputHexadecimal(string name) : this()
		{
			input = new WinFormsCtrlLibInputScreen.InputHexadecimal(name);
		}
		#endregion

		#region PpublicMethod
		public UserControlInput GetInputDisplay()
		{
			return input;
		}
		public IInput SetFont(Font font)
		{
			input.Font = font;

			return this;
		}
		public IInput SetName(string name)
		{
			input.Name = name;

			return this;
		}
		public IInput SetRange(Range<int> range)
		{
			input.MaxLength = range.Maximum;

			return this;
		}
		public IInput SetValue(string value)
		{
			input.Text = value;

			return this;
		}
		public IInput SetValue(string[] value)
		{
			throw new NotImplementedException();
		}
		public IInput SetValues(string value)
		{
			throw new NotImplementedException();
		}
		#endregion
	}
}
