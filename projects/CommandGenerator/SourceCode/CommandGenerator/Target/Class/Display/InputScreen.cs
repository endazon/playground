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
			Object = new InputScreenStorage.InputScreenObject(target);
		}

		public void Clear()
		{
			Object.Clear();
		}

		public void Save(List<CommandJsonStorage.Detail> items)
		{
			if (items == null) { return; }
			if (Object.Panel.Controls.Count <= 0) { return; }

			foreach (var group_item in Object.InputGroup)
			{
				int group_index = Object.InputGroup.IndexOf(group_item);
				foreach (var input_item in group_item.Input)
				{
					int input_index = group_item.Input.IndexOf(input_item);
					items[group_index].Controls[input_index].Value = input_item.InputBox.Text;
				}
			}
		}

		private List<InputScreenStorage.Input> ConvertDetail2InputList(CommandJsonStorage.Detail target)
		{
			int X                                       = 5;
			int Y                                       = 15;
			List<InputScreenStorage.Input> inputObjList = new List<InputScreenStorage.Input>();

			foreach (var parameter in target.Controls)
			{
				InputScreenStorage.Input inputObj = new InputScreenStorage.Input(
					parameter.Name,
					new Font("MS UI Gothic", 16F, FontStyle.Regular, GraphicsUnit.Point, 128),
					new Point(X, Y),
					parameter.Type,
					parameter.Size,
					parameter.Value
					);
				X = inputObj.InputBox.Location.X + inputObj.InputBox.Size.Width + 5;

				if (parameter.Fixed){ inputObj.Disabled();	}
				else				{ inputObj.Enabled();	}

				inputObjList.Add(inputObj);
			}

			return inputObjList;
		}

		private List<InputScreenStorage.InputGroup> ConvertDetails2InputGroup(List<CommandJsonStorage.Detail> target)
		{
			const int HEIGHT_SIZE = 50;
			const int SPACE_SIZE  = 10;
			int X                 = 3;
			int Y                 = 3;
			List<InputScreenStorage.InputGroup> inputGroupObjList = new List<InputScreenStorage.InputGroup>();

			foreach (var detail in target)
			{
				List<InputScreenStorage.Input> list = ConvertDetail2InputList(detail);
				int WIDTH_SIZE = list.Last().InputBox.Location.X
					    	   + list.Last().InputBox.Size.Width
						       + 10;

				InputScreenStorage.InputGroup inputGroupObj = new InputScreenStorage.InputGroup(
					detail.Name,
					 new Size(WIDTH_SIZE, HEIGHT_SIZE),
					 new Font("MS UI Gothic", 9F, FontStyle.Regular, GraphicsUnit.Point, 128),
					 new Point(X, Y + (inputGroupObjList.Count * HEIGHT_SIZE)
					 + (inputGroupObjList.Count * SPACE_SIZE))
					);
				inputGroupObj.SuspendLayout();
				inputGroupObj.Update(list);
				inputGroupObj.ResumeLayout(false);
				inputGroupObj.PerformLayout();

				inputGroupObjList.Add(inputGroupObj);
			}

			return inputGroupObjList;
		}

		public void Update(List<CommandJsonStorage.Detail> items)
		{
			if (items == null) { return; }

			Clear();
			Object.Update(ConvertDetails2InputGroup(items));
		}
	}
}