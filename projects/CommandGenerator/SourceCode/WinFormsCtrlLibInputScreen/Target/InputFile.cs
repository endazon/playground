using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Data;
using System.Drawing;
using System.IO;
using System.Text;
using System.Windows.Forms;

namespace WinFormsCtrlLibInputScreen
{
	public partial class InputFile : WinFormsCtrlLibInputScreen.InputString
	{
		#region Property
		public int MaxDisplayLength
		{
			get { return base.MaxLength; }
			set	{ base.MaxLength = value;}
		}
		#endregion

		#region Constructor
		public InputFile()
		{
			InitializeComponent();
		}
		public InputFile(string name) : this()
		{
			Name = name;
		}
		#endregion

		#region Helper
		protected override void UpdateDisplay()
		{
			base.UpdateDisplay();

			//Button
			Button.Location = new Point(Panel.Width, 0);
		}
		#endregion

		#region Event
		protected override void textBox_TextChanged(object sender, EventArgs e) { }
		protected override void TextBox_DragEnter(object sender, DragEventArgs e)
		{
			//ファイルがドラッグされている場合、カーソルを変更する
			if (e.Data.GetDataPresent(DataFormats.FileDrop))
			{
				e.Effect = DragDropEffects.Copy;
			}
		}
		protected override void TextBox_DragDrop(object sender, DragEventArgs e)
		{
			//ドロップされたファイルの一覧を取得
			string[] fileName = (string[])e.Data.GetData(DataFormats.FileDrop, false);
			if (fileName.Length <= 0)
			{
				return;
			}

			// ドロップ先がTextBoxであるかチェック
			TextBox txtTarget = sender as TextBox;
			if (txtTarget == null)
			{
				return;
			}

			//TextBoxの内容をファイル名に変更
			txtTarget.Text = fileName[0];
		}
		private void Button_Click(object sender, EventArgs e)
		{
			//ダイアログを表示する
			var OpenDialog = new OpenFileDialog();
			if (OpenDialog.ShowDialog() == DialogResult.OK)
			{
				//ファイル名取得(パス込み)
				Text = OpenDialog.FileName;
			}
		}
		#endregion
	}
}
