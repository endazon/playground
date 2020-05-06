using System.Collections.Generic;
using System.Windows.Forms;

namespace CommandGenerator.Class.Storage
{
	class InputScreenStorage
	{
		public class Input
		{
			public Label Label { get; set; } = new Label();
			public Control InputBox { get; set; } = new TextBox();

			public Control[] ToArray()
			{
				List<Control> result = new List<Control>();
				result.Add(Label);
				result.Add(InputBox);

				return result.ToArray();
			}
		}

		public class InputGroup
		{
			public GroupBox GroupBox { get; set; } = new GroupBox();
			public List<Input> Input { get; set; } = new List<Input>();
		}

		public class InputScreenObject
		{
			public SplitterPanel Panel { get; set; } = new SplitterPanel(null);
			public List<InputGroup> InputGroup { get; set; } = new List<InputGroup>();
		}
	}
}
