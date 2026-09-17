using CommandGenerator.Target.Common.Range;
using System;
using System.Drawing;
using System.Windows.Forms;
using WinFormsCtrlLibInputScreen.Base;

namespace CommandGenerator.Utility.InputDisplay.Adapter
{
	class InputSelection : IInput
	{
		private readonly WinFormsCtrlLibInputScreen.InputSelection input;

		#region Constructor
		public InputSelection() 
		{
			input = new WinFormsCtrlLibInputScreen.InputSelection();
		}
		public InputSelection(string name) : this()
		{
			input = new WinFormsCtrlLibInputScreen.InputSelection(name);
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
			throw new NotImplementedException();
		}
		public IInput SetValue(string value)
		{
			input.Text = value;

			return this;
		}
		public IInput SetValue(string[] value)
		{
			input.Items.AddRange(value);

			return this;
		}
		public IInput SetValues(string value)
		{
			input.Items.Add(value);

			return this;
		}
		#endregion
	}
}
