using System;
using System.Collections.Generic;
using System.Drawing;
using System.Windows.Forms;
using WinFormsCtrlLibInputScreen;
using WinFormsCtrlLibInputScreen.Base;

namespace CommandGenerator.Class.Storage
{
	class InputScreenStorage
	{
		public class Input
		{
			#region Property
			public UserControlInput InputBox { get; set; } = new UserControlInput();
			#endregion

			#region Constructor
			public Input()
			{

			}
			public Input(string text, Font font, Point point, string type, int size, string value) : this()
			{
				InputBox = CreateInputBox(type, text, size, value);
				InputBox.Font = font;
				InputBox.Location = point;
				InputBox.Tag = type;
			}
			#endregion

			#region Helper
			private UserControlInput CreateInputBox(string type, string name, int size, string value)
			{
				UserControlInput _input = null;
				switch (type)
				{
					case "FILE_SELECT":
						_input = new InputFile();
						((InputFile)_input).Name = name;
						((InputFile)_input).Text = value;
						break;
					case "SELECT":
						_input = new InputSelection();
						((InputSelection)_input).Name = name;
						((InputSelection)_input).Text = value;
						break;
					case "DEC":
						_input = new InputDecimal();
						((InputDecimal)_input).Minimum = 0;
						((InputDecimal)_input).Maximum = size;
						((InputDecimal)_input).Name = name;
						((InputDecimal)_input).Text = value;
						break;
					case "HEX":
						_input = new InputHexadecimal();
						((InputHexadecimal)_input).MaxLength = size;
						((InputHexadecimal)_input).Name = name;
						((InputHexadecimal)_input).Text = value;
						break;
					case "ASCII":
						_input = new InputString();
						((InputString)_input).MaxLength = size;
						((InputString)_input).Name = name;
						((InputString)_input).Text = value;
						break;
					default:
						throw  new Exception();
				}

				return _input;
			}
			#endregion

			#region PpublicMethod
			public Control[] ToArray()
			{
				List<Control> result = new List<Control>();
				result.Add(InputBox);

				return result.ToArray();
			}
			public void Enabled()
			{
				InputBox.Enabled = true;
			}
			public void Disabled()
			{
				InputBox.Enabled = false;
			}
			#endregion
		}

		public class InputGroup
		{
			#region Property
			public GroupBox GroupBox { get; set; } = new GroupBox();
			public List<Input> Input { get; set; } = new List<Input>();
			#endregion

			#region Constructor
			public InputGroup()
			{

			}
			public InputGroup(string text, Size size, Font font, Point point)
				: this()
			{
				GroupBox = CreateGroupBox(text, size, font, point);
			}
			#endregion

			#region Helper
			private GroupBox CreateGroupBox(string text, Size size, Font font, Point point)
			{
				GroupBox groupbox   = new GroupBox();
				//groupbox.AutoSize = true;
				groupbox.Font       = font;
				groupbox.ForeColor  = SystemColors.HighlightText;
				groupbox.Name       = text + groupbox.GetType().Name;
				groupbox.Text       = text;
				groupbox.Location   = point;
				groupbox.Size       = size;

				return groupbox;
			}
			#endregion

			#region PpublicMethod
			public void SuspendLayout()
			{
				GroupBox.SuspendLayout();
			}
			public void Update()
			{
				Update(Input);
			}
			public void Update(List<Input> inputList)
			{
				foreach (var item in inputList)
				{
					GroupBox.Controls.AddRange(item.ToArray());
				}
				Input = inputList;
			}
			public void ResumeLayout(bool performLayout)
			{
				GroupBox.ResumeLayout(performLayout);
			}
			public void PerformLayout()
			{
				GroupBox.PerformLayout();
			}
			public void LineUp()
			{
				LineUp(0);
			}
			public void LineUp(int span)
			{
				int offset = 0;
				foreach (var item in Input)
				{
					var _lable = new Label();
					_lable.AutoSize = true;
					_lable.Text = item.InputBox.Text;

					offset = offset < _lable.PreferredWidth ? _lable.PreferredWidth : offset;
				}
				foreach (var item in Input)
				{
					var _lable = new Label();
					_lable.AutoSize = true;
					_lable.Text = item.InputBox.Text;

					item.InputBox.LineUp((uint)(offset - _lable.PreferredWidth  + span));
				}
			}
			#endregion
		}

		public class InputScreenObject
		{
			#region Property
			public Panel Panel { get; set; } = new Panel();
			public List<InputGroup> InputGroup { get; set; } = new List<InputGroup>();
			#endregion

			#region Constructor
			public InputScreenObject()
			{

			}
			public InputScreenObject(Panel panel)
				: this()
			{
				Panel = panel;
			}
			#endregion

			#region Helper

			#endregion

			#region PpublicMethod
			public void Clear()
			{
				Panel.Controls.Clear();
				InputGroup.Clear();
			}
			public void Update()
			{
				Update(InputGroup);
			}
			public void Update(List<InputGroup> inputGroupList)
			{
				foreach (var item in inputGroupList)
				{
					Panel.Controls.Add(item.GroupBox);
				}
				InputGroup = inputGroupList;
			}
			#endregion
		}
	}
}
