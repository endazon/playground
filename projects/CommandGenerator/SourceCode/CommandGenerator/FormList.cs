using System;
using System.Text;
using System.Windows.Forms;

using System.IO;
using Newtonsoft.Json;
using CommandGenerator.Class.Storage;
using System.Drawing;
using CommandGenerator.Properties;
using CommandGenerator.Class.FileOperation;

namespace CommandGenerator
{
	public partial class FormList : Form
	{
		private CommandJsonStorage.CommandJsonObject CommandObj { get; set; } = new CommandJsonStorage.CommandJsonObject();
		private FormEdit FormEdit { get; set; } = new FormEdit();

		public FormList()
		{
			InitializeComponent();

			FormEdit = new FormEdit(CommandObj.Name, CommandObj.Version);
		}

		private void FormListShow()
		{
			//FormEdit = new FormEdit(CommandObj.Name, CommandObj.Version);
			FormEdit.LocationUpdate(Location.X + Size.Width, Location.Y);
			FormEdit.Show();
		}

		private void FormListEditorShowDialog()
		{
			var FormListEditor = new FormListEditor(CommandObj);
			FormListEditor.ShowDialog();
			CommandObj = (CommandJsonStorage.CommandJsonObject)FormListEditor.CommandObj;
		}

		private void Clear()
		{
			editFileOpenToolStripMenuItem.Enabled = false;
			FormEdit.Clear();
			FormEdit.Hide();
			CommandListBox.Items.Clear();
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

			foreach (var item in CommandObj.Items)
			{
				string displayName = item.Name;
				// リストボックスにアイテム追加 
				//for (int i = 0; i < 100; i++)
				CommandListBox.Items.Add(displayName);

				// リストボックスの一番上を選択
				CommandListBox.SetSelected(0, true);

				FormListShow();
				editFileOpenToolStripMenuItem.Enabled = true;
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
			var FormListEditor = new FormListEditor(CommandObj);
			FormListEditor.ShowDialog();
			CommandObj = (CommandJsonStorage.CommandJsonObject)FormListEditor.CommandObj;
			DisplayUpdate();
		}

		private void editFileOpenToolStripMenuItem_Click(object sender, EventArgs e)
		{
			FormListEditorShowDialog();
		}
		#endregion

		#region ListBox
		private void CommandListBox_Select(object sender, EventArgs e)
		{
			int index = ((ListBox)sender).SelectedIndex;

			if (index < 0) { return; }

			CommandJsonStorage.Item item = CommandObj.Items[index];

			FormEdit.add(item);
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
			Form obj = (Form)sender;
			FormEdit.LocationUpdate(obj.Location.X + obj.Size.Width, obj.Location.Y);
		}
		private void FormList_SizeChanged(object sender, EventArgs e)
		{
			Form obj = (Form)sender;
			FormEdit.LocationUpdate(obj.Location.X + obj.Size.Width, obj.Location.Y);
			FormEdit.SizeUpdate(-1, obj.Size.Height);
		}
		#endregion
	}
}
