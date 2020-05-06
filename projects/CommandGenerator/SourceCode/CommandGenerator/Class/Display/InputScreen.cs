using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;
using CommandGenerator.Class.Storage;

namespace CommandGenerator.Class.Display
{
	class InputScreen
	{
		private InputScreenStorage.InputScreenObject Object { get; set; } = new InputScreenStorage.InputScreenObject();

		public InputScreen(SplitterPanel target)
		{
			Object.Panel = target;
		}

		public void clear()
		{
			Object.Panel.Controls.Clear();
			Object.InputGroup.Clear();
		}

		public void save(List<CommandJsonStorage.Detail> items)
		{
			if (items == null) { return; }
			if (Object.Panel.Controls.Count <= 0) { return; }

			foreach (var group_item in Object.InputGroup)
			{
				int group_index = Object.InputGroup.IndexOf(group_item);
				foreach (var input_item in group_item.Input)
				{
					int input_index = group_item.Input.IndexOf(input_item);
					items[group_index].Parameter[input_index].Value = input_item.InputBox.Text;
				}
			}
		}

		private Size GetOneCharacterSize(Font font)
		{
			Label label    = new Label();
			label.AutoSize = true;
			label.Font     = font;
			label.Name     = " ";
			label.Text     = " ";
			return new Size(label.PreferredWidth - 5, label.PreferredHeight);
		}

		private Label GetLabelObj(CommandJsonStorage.Parameter item, Font font, Point point)
		{
			Label label       = new Label();
			label.AutoSize    = true;
			label.BorderStyle = BorderStyle.FixedSingle;
			label.Font        = font;
			label.Name        = item.Name;
			label.Text        = item.Name;
			label.Location    = point;
			label.Enabled     = !item.Fixed;
			return label;
		}

		private TextBox GetTextBoxObj(CommandJsonStorage.Parameter item, Font font, Point point, Size size)
		{
			TextBox textbox     = new TextBox();
			textbox.BackColor   = SystemColors.WindowText;
			textbox.BorderStyle = BorderStyle.FixedSingle;
			textbox.Font        = font;
			textbox.ForeColor   = SystemColors.GrayText;
			textbox.MaxLength   = 2 + (2 * item.Size);
			textbox.Name        = item.Value;
			textbox.Text        = item.Value;
			textbox.Location    = point;
			textbox.Size        = new Size(size.Width * textbox.MaxLength, size.Height);
			textbox.TextChanged+= new EventHandler(textBox_TextChanged);
			textbox.Enabled     = !item.Fixed;
			return textbox;
		}
		private void textBox_TextChanged(object sender, EventArgs e)
		{
			TextBox target = (TextBox)sender;
			string text    = (string)target.Text.Clone();
			int length     = text.Length;

			if ((length < 2)
				|| text.Substring(0, 1) != "0"
				|| text.Substring(1, 1) != "x")
			{
				target.Text = "0x";
				target.SelectionStart = target.TextLength;
				System.Media.SystemSounds.Asterisk.Play();
			}
			else if (length == 2) { return; }
			else
			{
				int char_pos = 2;
				foreach (var c in text.Substring(char_pos, length- char_pos))
				{
					if (!Uri.IsHexDigit(c))
					{
						target.Text = text.Remove(char_pos, 1);
						System.Media.SystemSounds.Asterisk.Play();
					}
					char_pos++;
				}
			}
		}

		private NumericUpDown GetNumericUpDownObj(CommandJsonStorage.Parameter item, Font font, Point point, Size size)
		{
			NumericUpDown numericupdown = new NumericUpDown();
			numericupdown.BackColor     = SystemColors.WindowText;
			numericupdown.BorderStyle   = BorderStyle.FixedSingle;
			numericupdown.Font          = font;
			numericupdown.ForeColor     = SystemColors.GrayText;
			numericupdown.Minimum       = 0;
			numericupdown.Maximum       = (int)Math.Pow(2, (8 * item.Size)) - 1;
			numericupdown.Name          = item.Value;
			numericupdown.Text          = item.Value;
			numericupdown.Location      = point;
			int digits                  = (int)Math.Log10((double)numericupdown.Maximum) + 1;
			numericupdown.Size          = new Size(size.Width * (digits + 1), size.Height);
			numericupdown.ValueChanged += new EventHandler(numericUpDown_ValueChanged);
			numericupdown.Enabled       = !item.Fixed;
			return numericupdown;
		}
		private void numericUpDown_ValueChanged(object sender, EventArgs e)
		{
			NumericUpDown target = (NumericUpDown)sender;
			decimal value        = target.Value;
			decimal maximum      = target.Maximum;

			if (maximum < value)
			{
				target.Value = maximum;
			}
		}

		private Control GetInputBoxObj(CommandJsonStorage.Parameter item, Font font, Point point, Size size)
		{
			Control result;

			switch (item.Type)
			{
				case "HEX":
					result = GetTextBoxObj(item, font, point, size);
					break;
				case "DEC":
				default:
					result = GetNumericUpDownObj(item, font, point, size);
					break;
			}

			return result;
		}

		private List<InputScreenStorage.Input> GetDetailObj(CommandJsonStorage.Detail target)
		{
			Font FONT                                   = new Font("MS UI Gothic", 16F, FontStyle.Regular, GraphicsUnit.Point, 128);
			int X                                       = 5;
			int Y                                       = 15;
			List<InputScreenStorage.Input> inputObjList = new List<InputScreenStorage.Input>();

			foreach (var parameter in target.Parameter)
			{
				InputScreenStorage.Input inputObj = new InputScreenStorage.Input();
				inputObj.Label    = GetLabelObj(parameter,
												FONT,
											    new Point(X, Y));
				inputObj.InputBox = GetInputBoxObj(parameter,
												   FONT,
												   new Point(X + inputObj.Label.PreferredWidth, Y),
												   new Size(GetOneCharacterSize(FONT).Width, inputObj.Label.PreferredHeight));
				X = inputObj.InputBox.Location.X + inputObj.InputBox.Size.Width + 5;

				inputObjList.Add(inputObj);
			}

			return inputObjList;
		}

		public void update(List<CommandJsonStorage.Detail> items)
		{
			if (items == null) { return; }

			clear();

			int WIDTH_SIZE;
			const int HEIGHT_SIZE = 50;
			const int SPACE_SIZE  = 10;
			int X                 = 3;
			int Y                 = 3;

			foreach (var detail in items)
			{
				InputScreenStorage.InputGroup inputObjGroup = new InputScreenStorage.InputGroup();
				inputObjGroup.GroupBox.SuspendLayout();

				inputObjGroup.Input = GetDetailObj(detail);
				foreach (var input in inputObjGroup.Input)
				{
					inputObjGroup.GroupBox.Controls.Add(input.Label);
					inputObjGroup.GroupBox.Controls.Add(input.InputBox);
				}
				WIDTH_SIZE = inputObjGroup.Input.Last().InputBox.Location.X 
					       + inputObjGroup.Input.Last().InputBox.Size.Width
						   + 10;
				inputObjGroup.GroupBox.ForeColor = SystemColors.HighlightText;
				inputObjGroup.GroupBox.Name      = detail.Name;
				inputObjGroup.GroupBox.TabIndex  = 2;
				inputObjGroup.GroupBox.TabStop   = false;
				inputObjGroup.GroupBox.Text      = detail.Name;
				inputObjGroup.GroupBox.Location  = new Point(X, Y + (Object.Panel.Controls.Count * HEIGHT_SIZE)
												 + (Object.Panel.Controls.Count * SPACE_SIZE));
				inputObjGroup.GroupBox.Size      = new Size(WIDTH_SIZE, HEIGHT_SIZE);
				inputObjGroup.GroupBox.ResumeLayout(false);
				inputObjGroup.GroupBox.PerformLayout();

				Object.Panel.Controls.Add(inputObjGroup.GroupBox);
				Object.InputGroup.Add(inputObjGroup);
			}
		}
	}
}