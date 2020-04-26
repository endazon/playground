using System;
using System.Collections.Generic;
using System.Text;
using System.Windows.Forms;

using System.IO;
using CommandGenerator.Class.Storage;
using CommandGenerator.Class.Display;
using System.Security.Permissions;
using CommandCreator.Class.Storage;
using System.Drawing;

namespace CommandGenerator
{
	public partial class FormEdit : Form
	{
		private CommandCsvStorage.CommandCsvObject CommandList { get; set; } = new CommandCsvStorage.CommandCsvObject();
		private List<CommandJsonStorage.Item> CommandItems { get; set; } = new List<CommandJsonStorage.Item>();
		private string GeneratesFileName { get; set; } = "";
		private InputScreen ScreenObj { get; set; } = new InputScreen(null);

		public FormEdit(string name, string ver)
		{
			InitializeComponent();
			CommandItems = new List<CommandJsonStorage.Item>();
			ScreenObj = new InputScreen(SplitContainer.Panel2);
			CommandList.Name = name;
			CommandList.Version = ver;
		}

		public FormEdit() : this("", ""){}

		//ウィンドウを閉じるボタン・ショートカットを無効化
		protected override CreateParams CreateParams
		{
			[SecurityPermission(SecurityAction.Demand,
				Flags = SecurityPermissionFlag.UnmanagedCode)]
			get
			{
				const int CS_NOCLOSE = 0x200;
				CreateParams cp = base.CreateParams;
				cp.ClassStyle = cp.ClassStyle | CS_NOCLOSE;

				return cp;
			}
		}

		public void LocationUpdate(int x, int y)
		{
			int buff_x = x >= 0 ? x : Location.X;
			int buff_y = y >= 0 ? y : Location.Y;

			Location = new Point(buff_x - 5, buff_y);
		}

		public void SizeUpdate(int width, int height)
		{
			int buff_width  = width >= 0  ? width  : Size.Width;
			int buff_height = height >= 0 ? height : Size.Height;

			Size = new Size(buff_width, buff_height);
		}

		public void clear()
		{
			CommandList.Name = "";
			CommandList.Version = "";
			CommandList.Items.Clear();
			CommandItems.Clear();
			GeneratesFileName = "";
			CommandListBox.Items.Clear();
			ScreenObj.clear();
		}

		internal void add(CommandJsonStorage.Item item)
		{
			CommandItems.Add(item.Clone());

			string displayName = item.Name + "(Len：" + item.Length + ")";
			// リストボックスにアイテム追加 
			//for (int i = 0; i < 100; i++)
			CommandListBox.Items.Add(displayName);

			// リストボックスの一番上を選択
			CommandListBox.SetSelected(CommandListBox.Items.Count - 1, true);
		}

		private void generator()
		{
			#region 前処理
			try
			{
				ScreenObj.save(CommandItems[CommandListBox.SelectedIndex].Detail);

				foreach (var item in CommandItems)
				{
					CommandCsvStorage.Detail item_obj = new CommandCsvStorage.Detail();
					item_obj.Name = item.Name;
					item_obj.Command = item.Parser();

					CommandList.Items.Add(item_obj);
				}
			}
			catch
			{
				return;
			}
			#endregion

			#region 生成先ファイル選択
			try
			{
				if (GeneratesFileName == "")
				{
					SaveFileDialog genFile = new SaveFileDialog();
					string fileName = "";

					//初期値
					genFile.FileName = "default.csv";

					//ファイルの指定
					genFile.Filter = "CSVファイル(*.csv)|*.csv";

					//タイトルの設定
					genFile.Title = "保存先を指定してください";

					//ダイアログボックスを閉じる前に現在のディレクトリを復元するようにする
					genFile.RestoreDirectory = true;

					//ダイアログを表示する
					if (genFile.ShowDialog() == DialogResult.OK)
					{
						//ファイル名取得(パス込み)
						fileName = genFile.FileName;
					}
					//Console.WriteLine(fileName);

					GeneratesFileName = fileName;
				}
			}
			catch
			{
				return;
			}
			#endregion

			#region CSVファイル書き込み
			try
			{
				// 出力用のファイルを開く
				using (var sw = new StreamWriter(@GeneratesFileName, false, Encoding.UTF8))
				{
					sw.WriteLine("{0}, {1}", "Name", CommandList.Name);
					sw.WriteLine("{0}, {1}", "Version", CommandList.Version);
					sw.WriteLine(",");
					sw.WriteLine("No., Name, Command");
					foreach (var list in CommandList.Items)
					{
						sw.WriteLine("{0}, {1}, {2},", (CommandList.Items.IndexOf(list) + 1), list.Name, list.Command);
						//Console.WriteLine(list.Name + "：" + list.Command);
					}
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
				CommandList.Items.Clear();
			}
			#endregion
		}

		#region Menu
		private void newToolStripMenuItem_Click(object sender, EventArgs e)
		{
			clear();
		}

		private void saveToolStripMenuItem_Click(object sender, EventArgs e)
		{
			generator();
		}

		private void saveAsToolStripMenuItem_Click(object sender, EventArgs e)
		{
			GeneratesFileName = "";
			generator();
		}
		#endregion

		#region ListBox
		private void CommandListBox_DoubleClick(object sender, EventArgs e)
		{
			int index = ((ListBox)sender).SelectedIndex;
			CommandItems.RemoveAt(index);
			CommandListBox.Items.RemoveAt(index);
			ScreenObj.clear();
		}

		private List<CommandJsonStorage.Detail> z1Items = null;
		private void CommandListBox_SelectedIndexChanged(object sender, EventArgs e)
		{
			int index = ((ListBox)sender).SelectedIndex;

			if(index < 0) { return; }

			List<CommandJsonStorage.Detail> items = CommandItems[index].Detail;

			ScreenObj.save(z1Items);
			ScreenObj.update(items);

			z1Items = items;
		}

		#endregion

		#region Button
		private void buttonGenerates_Click(object sender, EventArgs e)
		{
			generator();
		}
		#endregion
	}
}
