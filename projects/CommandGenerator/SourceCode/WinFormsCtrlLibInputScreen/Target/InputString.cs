using System;
using System.Drawing;
using System.Windows.Forms;

namespace WinFormsCtrlLibInputScreen
{
	public partial class InputString :  Base.UserControlInput
	{
		#region Property
		public override string Text
		{
			get { return TextBox.Text; }
			set { TextBox.Text = value; }
		}
		public virtual int MaxLength
		{
			get { return TextBox.MaxLength;  }
			set { TextBox.MaxLength = value; }
		}
		#endregion

		#region Constructor
		public InputString()
		{
			InitializeComponent();
		}
		public InputString(string name) : this()
		{
			Name = name;
		}
		#endregion

		#region Helper
		protected Size GetOneCharacterSize(Font font)
		{
			//表示先
			var PictureBox = new PictureBox();
			//表示する文字列
			var s = "A";
			//描画先とするImageオブジェクトを作成する
			var canvas = new Bitmap(PictureBox.Width, PictureBox.Height);
			//ImageオブジェクトのGraphicsオブジェクトを作成する
			var g = Graphics.FromImage(canvas);

			//NoPaddingにして、文字列を描画する
			TextRenderer.DrawText(g, s, font, new Point(0, 50), Color.Black, TextFormatFlags.NoPadding);
			//大きさを計測して、サイズを返す
			return TextRenderer.MeasureText(g, s, font, new Size(1000, 1000), TextFormatFlags.NoPadding);
		}
		protected override void UpdateDisplay()
		{
			var font_size = GetOneCharacterSize(TextBox.Font);

			//TextBox
			TextBox.Size = new Size(font_size.Width * TextBox.MaxLength, TextBox.Height);
		}
		#endregion

		#region Event
		protected virtual void textBox_TextChanged(object sender, EventArgs e) { }
		protected virtual void TextBox_DragEnter(object sender, DragEventArgs e) { }
		protected virtual void TextBox_DragDrop(object sender, DragEventArgs e) { }
		#endregion
	}
}