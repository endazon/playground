using System;
using System.Text;
using System.Windows.Forms;

using System.IO;
using Newtonsoft.Json;
using CommandGenerator.Class.Storage;
using System.Drawing;

namespace CommandGenerator
{
	public partial class FormList : Form
	{
		private string FileName { get; set; } = "";
		internal CommandJsonStorage.CommandJsonObject CommandObj { get; set; } = new CommandJsonStorage.CommandJsonObject();

		private FormEdit FormEdit { get; set; }
		private FormListEditor FormListEditor { get; set; }

		public FormList()
		{
			InitializeComponent();
			FormEdit       = new FormEdit(this);
			FormListEditor = new FormListEditor(this);
		}

		private void FormListShow()
		{
			FormEdit.LocationUpdate(Location.X + Size.Width, Location.Y);
			FormEdit.Show();
		}

		private void clear()
		{
			FileName = "";
			editFileOpenToolStripMenuItem.Enabled = false;
			CommandObj = new CommandJsonStorage.CommandJsonObject();
			CommandListBox.Items.Clear();
			FormEdit.clear();
			FormEdit.Hide();
		}

		private void open()
		{
			#region 前処理
			{
				clear();
			}
			#endregion

			#region JSONファイル選択
			try
			{
				string fileName = "";

				//ダイアログを表示する
				if (openFileDialogJson.ShowDialog() == DialogResult.OK)
				{
					//ファイル名取得(パス込み)
					fileName = openFileDialogJson.FileName;
				}
				//Console.WriteLine(fileName);

				FileName = fileName;
			}
			catch
			{
				clear();
			}
			#endregion

			#region JSONファイル読み込み
			try
			{
				CommandJsonStorage.CommandJsonObject obj = null;

				if (!string.IsNullOrWhiteSpace(FileName))
				{
					//ファイルを UTF-8 で開く
					using (var sr = new StreamReader(@FileName, Encoding.UTF8))
					{
						// 変数 jsonText にファイルの内容を代入 
						var jsonText = sr.ReadToEnd();

						// インスタンス CommandtStorage にデシリアライズ
						obj = JsonConvert.DeserializeObject<CommandJsonStorage.CommandJsonObject>(jsonText);
					}
				}
				//Console.WriteLine(JsonConvert.SerializeObject(obj));

				CommandObj = obj;
			}
			catch
			{
				clear();
			}
			#endregion

			#region 後処理
			try
			{
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
			catch
			{
				clear();
			}
			#endregion
		}

		private void save()
		{
			#region 前処理
			{

			}
			#endregion

			#region 保存先ファイル選択
			try
			{
				if (FileName == "")
				{
					string fileName = "";

					//ダイアログを表示する
					if (saveFileDialogJson.ShowDialog() == DialogResult.OK)
					{
						//ファイル名取得(パス込み)
						fileName = saveFileDialogJson.FileName;
					}
					//Console.WriteLine(fileName);

					FileName = fileName;
				}
			}
			catch
			{
				return;
			}
			#endregion

			#region JSONファイル書き込み
			try
			{
				// 出力用のファイルを開く
				using (var sw = new StreamWriter(FileName, false, Encoding.UTF8))
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
			#endregion

			#region 後処理
			{

			}
			#endregion
		}

		#region Menu
		private void newToolStripMenuItem_Click(object sender, EventArgs e)
		{
			clear();
			FormListEditor.ShowDialog();
		}

		private void openToolStripMenuItem_Click(object sender, EventArgs e)
		{
			open();
		}

		private void closeToolStripMenuItem_Click(object sender, EventArgs e)
		{
			clear();
		}

		private void saveToolStripMenuItem_Click(object sender, EventArgs e)
		{
			save();
		}

		private void saveAsToolStripMenuItem_Click(object sender, EventArgs e)
		{
			FileName = "";
			save();
		}

		private void endToolStripMenuItem_Click(object sender, EventArgs e)
		{
			clear();
			Close();
		}
		
		private void listToolStripMenuItem_Click(object sender, EventArgs e)
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
