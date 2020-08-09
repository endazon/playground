using System;
using System.Windows.Forms;
using System.Drawing;

using ListSettings;
using CommandGenerator.Class.Storage;
using CommandGenerator.Properties;
using CommandGenerator.Class.FileOperation;

namespace CommandGenerator
{
	public partial class FormMain : Form
	{
		private CommandJsonStorage.CommandJsonObject CommandObj { get; set; } = new CommandJsonStorage.CommandJsonObject();
		private FormEdit FormEdit { get; set; } = null;

		public FormMain()
		{
			InitializeComponent();
		}

		private FormEdit FormListCreateInstance()
		{
			var form = new FormEdit();
			if (generateToolStripMenuItem.Checked)
			{
				form = new FormGenerate(CommandObj.Name, CommandObj.Version);
			}
			else if (communicationToolStripMenuItem.Checked)
			{
				form = new FormCommunication();
			}
			else
			{
				throw new Exception();
			}

			form.SizeUpdate(-1, Size.Height);
			form.LocationUpdate(Location.X + Size.Width, Location.Y);

			return form;
		}
		private void FormListShow()
		{
			if (FormEdit != null) return;

			FormEdit = FormListCreateInstance();
			FormEdit.Show(this);
			FormEdit.LocationUpdate(Location.X + Size.Width, Location.Y);
		}
		private void FormListClose()
		{
			if (FormEdit == null) return;

			FormEdit.Close();
			FormEdit = null;
		}

		private void FormListEditorShowDialog()
		{
			var FormListEditor = new ListSetting(CommandObj);
			FormListEditor.ShowDialog(this);
			CommandObj = (CommandJsonStorage.CommandJsonObject)FormListEditor.GetListData();
		}

		private void Clear()
		{
			editFileOpenToolStripMenuItem.Enabled = false;
			CommandListBox.Items.Clear();
			FormListClose();
		}

		private void Init()
		{
			Clear();
			CommandObj = new CommandJsonStorage.CommandJsonObject();
			Settings.Default.JsonFilePath = "";
		}

		private void DisplayUpdate()
		{
			// 設定を初期化
			Clear();

			foreach (var item in CommandObj.Controls)
			{
				if (item.Verification()) {
					string displayName = item.Name;
					// リストボックスにアイテム追加 
					//for (int i = 0; i < 100; i++)
					CommandListBox.Items.Add(displayName);

					// リストボックスの一番上を選択
					CommandListBox.SetSelected(0, true);

					FormListShow();
					editFileOpenToolStripMenuItem.Enabled = true;
				}
				else
				{
					MessageBox.Show(item.Name+"の登録に失敗しました。\n設定を確認してください、",
									"エラー",
									MessageBoxButtons.OK,
									MessageBoxIcon.Error);
				}
			}
		}

		#region Menu
		private void newToolStripMenuItem_Click(object sender, EventArgs e)
		{
			Init();
			FormListEditorShowDialog();
			DisplayUpdate();
		}

		private void openToolStripMenuItem_Click(object sender, EventArgs e)
		{
			var JsonFile = new JsonFile<CommandJsonStorage.CommandJsonObject>(Settings.Default.JsonFilePath);
			if (JsonFile.Open())
			{
				Settings.Default.JsonFilePath = JsonFile.FileName;
				CommandObj                    = JsonFile.Object;
				Clear();
				DisplayUpdate();
			}
		}

		private void closeToolStripMenuItem_Click(object sender, EventArgs e)
		{
			Init();
		}

		private void saveToolStripMenuItem_Click(object sender, EventArgs e)
		{
			var JsonFile = new JsonFile<CommandJsonStorage.CommandJsonObject>(Settings.Default.JsonFilePath, CommandObj);
			if (JsonFile.FileName != "")
			{
				JsonFile.Write();
			}
			else
			{
				JsonFile.Save();
			}
			Settings.Default.JsonFilePath = JsonFile.FileName;
		}

		private void saveAsToolStripMenuItem_Click(object sender, EventArgs e)
		{
			var JsonFile = new JsonFile<CommandJsonStorage.CommandJsonObject>(Settings.Default.JsonFilePath, CommandObj);
			if (JsonFile.Save())
			{
				Settings.Default.JsonFilePath = JsonFile.FileName;
			}
		}

		private void endToolStripMenuItem_Click(object sender, EventArgs e)
		{
			Close();
		}

		private void commandListToolStripMenuItem_Click(object sender, EventArgs e)
		{
			FormListEditorShowDialog();
			DisplayUpdate();
		}

		private void editFileOpenToolStripMenuItem_Click(object sender, EventArgs e)
		{
			var csvFile = new CsvFile();
			if (csvFile.Open())
			{
				FormListClose();
				FormListShow();
				FormEdit.Update(CommandObj.Parser(csvFile.Object));
			}
		}

		private void modeToolStripMenuItem_Click(object sender, EventArgs e)
		{
			ToolStripMenuItem toolStripMenuItem = (ToolStripMenuItem)sender;

			toolStripMenuItem.Checked = true;
			toolStripMenuItem.CheckState = CheckState.Indeterminate;
			foreach (var release in (ToolStripMenuItem[])toolStripMenuItem.Tag)
			{
				release.Checked = false;
				release.CheckState = CheckState.Unchecked;
			}
		}
		
		private void modeToolStripMenuItem_CheckedChanged(object sender, EventArgs e)
		{
			FormListClose();
			FormListShow();
		}
		#endregion

		#region ListBox
		private void CommandListBox_Select(object sender, EventArgs e)
		{
			if (FormEdit == null) return;

			int index = ((ListBox)sender).SelectedIndex;

			if (index < 0) { return; }

			CommandJsonStorage.Item item = CommandObj.Controls[index];

			FormEdit.Add(item);
		}
		private void CommandListBox_DoubleClick(object sender, EventArgs e)
		{
			CommandListBox_Select(sender, e);
		}
		private void CommandListBox_KeyDown(object sender, KeyEventArgs e)
		{
			if (e.KeyCode == Keys.Enter)
			{
				CommandListBox_Select(sender, e);
			}
		}
		#endregion

		#region Window
		private void FormList_Load(object sender, EventArgs e)
		{
			var JsonFile = new JsonFile<CommandJsonStorage.CommandJsonObject>(Settings.Default.JsonFilePath);
			JsonFile.Read();
			if (JsonFile.Object != null) {
				CommandObj = JsonFile.Object;
			}

			DisplayUpdate();
		}
		private void FormList_FormClosed(object sender, FormClosedEventArgs e)
		{
			Clear();
			Settings.Default.Save();
		}
		private void FormList_LocationChanged(object sender, EventArgs e)
		{
			if (FormEdit == null) return;

			Form obj = (Form)sender;
			FormEdit.LocationUpdate(obj.Location.X + obj.Size.Width, obj.Location.Y);
		}
		private void FormList_SizeChanged(object sender, EventArgs e)
		{
			if (FormEdit == null) return;

			Form obj = (Form)sender;

			FormEdit.WindowState = obj.WindowState;
			FormEdit.SizeUpdate(-1, obj.Size.Height);
			FormEdit.LocationUpdate(obj.Location.X + obj.Size.Width, obj.Location.Y);
		}
		#endregion
	}
}
