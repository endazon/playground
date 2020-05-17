using System;
using System.Text;
using System.Windows.Forms;

using System.IO;
using Newtonsoft.Json;
using CommandGenerator.Class.Storage;
using System.Drawing;
using CommandGenerator.Properties;

namespace CommandGenerator
{
	public partial class FormList : Form
	{
		private Settings settings { get; set; } = new Settings();
		internal CommandJsonStorage.CommandJsonObject CommandObj { get; set; } = new CommandJsonStorage.CommandJsonObject();

		private FormEdit FormEdit { get; set; }
		private FormListEditor FormListEditor { get; set; }

		public FormList()
		{
			InitializeComponent();
			FormEdit       = new FormEdit(this);
			FormListEditor = new FormListEditor(this);
			settings       = Settings.Default;
		}

		private void FormListShow()
		{
			FormEdit.LocationUpdate(Location.X + Size.Width, Location.Y);
			FormEdit.Show();
		}

		private void clear()
		{
			editFileOpenToolStripMenuItem.Enabled = false;
			CommandObj = new CommandJsonStorage.CommandJsonObject();
			CommandListBox.Items.Clear();
			FormEdit.clear();
			FormEdit.Hide();
		}

		private bool selectJsonFile()
		{
			bool result = false;
			try
			{
				//ダイアログを表示する
				if (openFileDialogJson.ShowDialog() == DialogResult.OK)
				{
					//ファイル名取得(パス込み)
					settings.JsonFilePath = openFileDialogJson.FileName;
					result = true;
				}
				//Console.WriteLine(fileName);
			}
			catch
			{
				clear();
			}

			return result;
		}

		private void readJsonFile()
		{
			try
			{
				//ファイルを UTF-8 で開く
				using (var sr = new StreamReader(@settings.JsonFilePath, Encoding.UTF8))
				{
					// 変数 jsonText にファイルの内容を代入 
					var jsonText = sr.ReadToEnd();

					// インスタンス CommandtStorage にデシリアライズ
					CommandObj = JsonConvert.DeserializeObject<CommandJsonStorage.CommandJsonObject>(jsonText);

					// コマンド編集画面初期化
					CommandListBox.Items.Clear();
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
				//Console.WriteLine(JsonConvert.SerializeObject(obj));
			}
			catch
			{
				clear();
			}
		}

		private void writeJsonFile()
		{
			try
			{
				// 出力用のファイルを開く
				using (var sw = new StreamWriter(@settings.JsonFilePath, false, Encoding.UTF8))
				{
					// 変数 jsonText に CommandObj にシリアライズした内容を代入 
					var jsonText = JsonConvert.SerializeObject(CommandObj);

					// 変数 jsonText の内容をファイルに書き込む
					sw.Write(jsonText);
					sw.Flush();
				}
			}
			catch (Exception e)
			{
				// ファイルを開くのに失敗したときエラーメッセージを表示
				Console.WriteLine(e.Message);
				return;
			}
		}

		#region Menu
		private void newToolStripMenuItem_Click(object sender, EventArgs e)
		{
			clear();
			FormListEditor.ShowDialog();
		}

		private void openToolStripMenuItem_Click(object sender, EventArgs e)
		{
			if (selectJsonFile())
			{
				clear();
				readJsonFile();
			}
		}

		private void closeToolStripMenuItem_Click(object sender, EventArgs e)
		{
			clear();
		}

		private void saveToolStripMenuItem_Click(object sender, EventArgs e)
		{
			writeJsonFile();
		}

		private void saveAsToolStripMenuItem_Click(object sender, EventArgs e)
		{
			if (selectJsonFile())
			{
				writeJsonFile();
			}
		}

		private void endToolStripMenuItem_Click(object sender, EventArgs e)
		{
			clear();
			Close();
		}

		private void commandListToolStripMenuItem_Click(object sender, EventArgs e)
		{
			FormListEditor.ShowDialog();
		}

		private void editFileOpenToolStripMenuItem_Click(object sender, EventArgs e)
		{

		}
		#endregion

		#region ListBox
		private void CommandListBox_DoubleClick(object sender, EventArgs e)
		{
			int index = ((ListBox)sender).SelectedIndex;

			if (index < 0) { return; }

			CommandJsonStorage.Item item = CommandObj.Items[index];

			FormEdit.add(item);
		}
		#endregion

		#region Window
		private void FormList_Load(object sender, EventArgs e)
		{
			readJsonFile();
		}

		private void FormList_FormClosed(object sender, FormClosedEventArgs e)
		{
			settings.Save();
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

		public void FormListEditor_FormClosed(object sender, FormClosedEventArgs e)
		{
			CommandListBox.Items.Clear();
			FormEdit.clear();
			FormEdit.Hide();

			if (CommandObj != null)
			{
				CommandListBox.Items.Clear();
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
		}
		#endregion
	}
}
