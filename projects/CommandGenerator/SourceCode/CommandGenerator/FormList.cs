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
		private FormEdit FormEdit { get; set;  } = new FormEdit();
		private string CommandFileName { get; set; } = "";
		private CommandJsonStorage.CommandJsonObject CommandObj { get; set; } = new CommandJsonStorage.CommandJsonObject();

		public FormList()
		{
			InitializeComponent();
		}

		private void FormListShow()
		{
			FormEdit.LocationUpdate(Location.X + Size.Width, Location.Y);
			FormEdit.Show();
		}

		private void clear()
		{
			CommandFileName = "";
			CommandObj = new CommandJsonStorage.CommandJsonObject();
			CommandListBox.Items.Clear();
			FormEdit.clear();
			FormEdit.Close();
		}

		#region Menu
		private void newToolStripMenuItem_Click(object sender, EventArgs e)
		{

		}

		private void openToolStripMenuItem_Click(object sender, EventArgs e)
		{
			#region 前処理
			{

			}
			#endregion

			#region JSONファイル選択
			try
			{
				OpenFileDialog cmdFile = new OpenFileDialog();
				string fileName = "";

				//初期値
				cmdFile.FileName = "default.json";

				//ファイルの指定
				cmdFile.Filter = "JSONファイル(*.json)|*.json";

				//タイトルの設定
				cmdFile.Title = "開くコマンドファイルを選択してください";

				//ダイアログボックスを閉じる前に現在のディレクトリを復元するようにする
				cmdFile.RestoreDirectory = true;

				//ダイアログを表示する
				if (cmdFile.ShowDialog() == DialogResult.OK)
				{
					//ファイル名取得(パス込み)
					fileName = cmdFile.FileName;
				}
				//Console.WriteLine(fileName);

				CommandFileName = fileName;
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

				if (!string.IsNullOrWhiteSpace(CommandFileName))
				{
					//ファイルを UTF-8 で開く
					using (var sr = new StreamReader(@CommandFileName, Encoding.UTF8))
					{
						// 変数 jsonText にファイルの内容を代入 
						var jsonText = sr.ReadToEnd();

						// インスタンス CommandtStorage にデシリアライズ
						obj = JsonConvert.DeserializeObject<CommandJsonStorage.CommandJsonObject>(jsonText);
					}
					
					FormEdit = new FormEdit(obj.Name, obj.Version);
				}
				//Console.WriteLine(JsonConvert.SerializeObject(obj));

				CommandObj = obj;
			}
			catch
			{
				clear();
			}
			#endregion

			#region コマンド表示
			try
			{
				if (CommandObj != null)
				{
					CommandListBox.Items.Clear();
					foreach (var item in CommandObj.Items)
					{
						string displayName = item.Name + "(Len：" + item.Length + ")";
						// リストボックスにアイテム追加 
						//for (int i = 0; i < 100; i++)
						CommandListBox.Items.Add(displayName);

						// リストボックスの一番上を選択
						CommandListBox.SetSelected(0, true);

						FormListShow();
					}
				}
			}
			catch
			{
				clear();
			}
			#endregion

			#region 後処理
			{

			}
			#endregion
		}

		private void closeToolStripMenuItem_Click(object sender, EventArgs e)
		{
			clear();
		}

		private void saveToolStripMenuItem_Click(object sender, EventArgs e)
		{

		}

		private void saveAsToolStripMenuItem_Click(object sender, EventArgs e)
		{

		}

		private void endToolStripMenuItem_Click(object sender, EventArgs e)
		{
			clear();
			Close();
		}
		#endregion

		#region ListBox
		private void CommandListBox_DoubleClick(object sender, EventArgs e)
		{
			int index = ((ListBox)sender).SelectedIndex;
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
		#endregion
	}
}
