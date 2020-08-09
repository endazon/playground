using System.Windows.Forms;
using static CommandGenerator.Class.Storage.CommandJsonStorage;

namespace ListSettings
{
	class ListSetting : IListSetting
	{
		private readonly FormMain Form;
		private CommandJsonObject Data;

		public ListSetting(CommandJsonObject srcDataObj)
		{
			Data = srcDataObj;
			Form = new FormMain(Data.Name, Data.Version);
			Form.CommandObj = Data;
		}
		public ListSetting() : this(new CommandJsonObject())
		{
			Data = new CommandJsonObject();
		}
		
		public CommandJsonObject GetListData()
		{
			Data = (CommandJsonObject)Form.CommandObj;
			return Data;
		}

		public DialogResult ShowDialog()
		{
			return ShowDialog(null);
		}
		public DialogResult ShowDialog(IWin32Window owner)
		{
			return Form.ShowDialog(owner);
		}
	}
}
