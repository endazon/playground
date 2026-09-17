using System.Windows.Forms;
using static CommandGenerator.Class.Storage.CommandJsonStorage;

namespace ListSettings
{
	interface IListSetting
	{
		CommandJsonObject GetListData();
		DialogResult ShowDialog();
		DialogResult ShowDialog(IWin32Window owner);
	}
}
