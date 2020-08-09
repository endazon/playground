using CommandGenerator.Class.Storage;
using Newtonsoft.Json;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace CommandGenerator.Class.FileOperation
{
	class CsvFile
	{
		private OpenFileDialog OpenDialog { get; } = new OpenFileDialog();
		private SaveFileDialog SaveDialog { get; } = new SaveFileDialog();
		public string FileName { get; set; } = "";
		public CommandCsvStorage.CommandCsvObject Object { get; set; } = new CommandCsvStorage.CommandCsvObject();

		public CsvFile()
		{
			InitializeComponent();
		}
		public CsvFile(string fn) : this()
		{
			FileName = fn;
		}
		public CsvFile(string fn, object obj) : this(fn)
		{
			Object = (CommandCsvStorage.CommandCsvObject)obj;
		}

		private void InitializeComponent()
		{
			// 
			// openFileDialogJson
			// 
			OpenDialog.FileName         = "default.csv";
			OpenDialog.Filter           = "CSVファイル(*.csv)|*.csv";
			OpenDialog.RestoreDirectory = true;
			OpenDialog.Title            = "開くコマンドファイルを選択してください";
			// 
			// saveFileDialogJson
			// 
			SaveDialog.FileName         = "default.csv";
			SaveDialog.Filter           = "CSVファイル(*.csv)|*.csv";
			SaveDialog.RestoreDirectory = true;
			SaveDialog.Title            = "保存先を指定してください";
		}

		public void Clear()
		{
			FileName = null;
			Object = new CommandCsvStorage.CommandCsvObject();
		}

		public bool Open()
		{
			bool result = false;
			try
			{
				//ダイアログを表示する
				if (OpenDialog.ShowDialog() == DialogResult.OK)
				{
					//ファイル名取得(パス込み)
					FileName = OpenDialog.FileName;
					Read();
					result = true;
				}
				//Console.WriteLine(fileName);
			}
			catch (Exception e)
			{
				// ファイルを開くのに失敗したときエラーメッセージを表示
				Console.WriteLine(e.Message);
			}

			return result;
		}

		public bool Save()
		{
			bool result = false;
			try
			{
				//ダイアログを表示する
				if (SaveDialog.ShowDialog() == DialogResult.OK)
				{
					//ファイル名取得(パス込み)
					FileName = SaveDialog.FileName;
					Write();
					result = true;
				}
				//Console.WriteLine(fileName);
			}
			catch (Exception e)
			{
				// ファイルを開くのに失敗したときエラーメッセージを表示
				Console.WriteLine(e.Message);
			}

			return result;
		}

		public void Read()
		{
			try
			{
				//ファイルを UTF-8 で開く
				using (var sr = new StreamReader(@FileName, Encoding.UTF8))
				{
					// 変数 jsonText にファイルの内容を代入 
					var csvText = sr.ReadToEnd();

					// インスタンス CommandtStorage にデシリアライズ
					Object.Deserialize(csvText);
				}
				//Console.WriteLine(JsonConvert.SerializeObject(obj));
			}
			catch (Exception e)
			{
				// ファイルを開くのに失敗したときエラーメッセージを表示
				Console.WriteLine(e.Message);
			}
		}

		public void Write()
		{
			try
			{
				// 出力用のファイルを開く
				using (var sw = new StreamWriter(@FileName, false, Encoding.UTF8))
				{
					// 変数 jsonText に CommandObj にシリアライズした内容を代入 
					var csvText = Object.Serialize();

					// 変数 jsonText の内容をファイルに書き込む
					sw.Write(csvText);
					sw.Flush();
				}
			}
			catch (Exception e)
			{
				// ファイルを開くのに失敗したときエラーメッセージを表示
				Console.WriteLine(e.Message);
			}
		}
	}
}
