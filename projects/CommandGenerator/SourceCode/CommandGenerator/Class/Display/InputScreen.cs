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

		private Label GetLabelObj(string param, float font_size, Point point)
		{
			Label label       = new Label();
			label.AutoSize    = true;
			label.BorderStyle = BorderStyle.FixedSingle;
			label.Font        = new Font("MS UI Gothic", font_size, FontStyle.Regular, GraphicsUnit.Point, 128);
			label.Name        = param;
			label.Text        = param;
			label.Location    = point;
			return label;
		}

		private TextBox GetTextBoxObj(string param, float font_size, Point point, Size size)
		{
			TextBox textbox     = new TextBox();
			textbox.BackColor   = SystemColors.WindowText;
			textbox.BorderStyle = BorderStyle.FixedSingle;
			textbox.Font        = new Font("MS UI Gothic", font_size, FontStyle.Regular, GraphicsUnit.Point, 128);
			textbox.ForeColor   = SystemColors.GrayText;
			textbox.MaxLength   = 4;
			textbox.Name        = param;
			textbox.Text        = param;
			textbox.Location    = point;
			textbox.Size        = new Size(size.Width * 4, size.Height);
			return textbox;
		}

		private NumericUpDown GetNumericUpDownObj(string param, float font_size, Point point, Size size)
		{
			NumericUpDown numericupdown = new NumericUpDown();
			numericupdown.BackColor     = SystemColors.WindowText;
			numericupdown.BorderStyle   = BorderStyle.FixedSingle;
			numericupdown.Font          = new Font("MS UI Gothic", font_size, FontStyle.Regular, GraphicsUnit.Point, 128);
			numericupdown.ForeColor     = SystemColors.GrayText;
			numericupdown.Minimum       = 0;
			numericupdown.Maximum       = 255;
			numericupdown.Name          = param;
			numericupdown.Text          = param;
			numericupdown.Location      = point;
			numericupdown.Size          = new Size(size.Width * 4, size.Height);
			return numericupdown;
		}

		private Control GetInputBoxObj(string type, string param, float font_size, Point point, Size size)
		{
			Control result;

			switch (type)
			{
				case "HEX":
					result = GetTextBoxObj(param, font_size, point, size);
					break;
				case "DEC":
				default:
					result = GetNumericUpDownObj(param, font_size, point, size);
					break;
			}

			return result;
		}

		private List<InputScreenStorage.Input> GetDetailObj(CommandJsonStorage.Detail target)
		{
			const float FONT_SIZE                       = 16F;
			int X                                       = 5;
			int Y                                       = 15;
			List<InputScreenStorage.Input> inputObjList = new List<InputScreenStorage.Input>();

			foreach (var parameter in target.Parameter)
			{
				InputScreenStorage.Input inputObj = new InputScreenStorage.Input();
				inputObj.Label    = GetLabelObj(parameter.Name,
											    FONT_SIZE,
											    new Point(X, Y));
				inputObj.InputBox = GetInputBoxObj(parameter.Type,
												   parameter.Value,
												   FONT_SIZE,
												   new Point(X + inputObj.Label.PreferredWidth, Y),
												   new Size((int)FONT_SIZE, inputObj.Label.PreferredHeight));
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