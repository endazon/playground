using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Drawing;
using System.Data;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace WinFormsCtrlLibInputScreen
{
	public partial class InputSelection : Base.UserControlInput
	{
		#region Property
		public override string Text
		{
			get { return ComboBox.Text; }
			set { ComboBox.Text = value; }
		}
		public ComboBox.ObjectCollection Items
		{
			get { return ComboBox.Items; }
		}
		#endregion

		#region Constructor
		public InputSelection()
		{
			InitializeComponent();
		}
		public InputSelection(string name) : this()
		{
			Name = name;
		}
		#endregion

		#region Helper
		private Size GetOneCharacterSize(Font font)
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
			var font_size = GetOneCharacterSize(ComboBox.Font);

			//ComboBox
			var max_length = Text.Length;
			foreach(object item in Items)
			{
				var _len = item.ToString().Length;
				max_length = _len > max_length ? _len : max_length;
			}
			ComboBox.Size = new Size(font_size.Width * max_length + 20, ComboBox.Height);
		}
		#endregion

		#region Event

		#endregion
	}
}
