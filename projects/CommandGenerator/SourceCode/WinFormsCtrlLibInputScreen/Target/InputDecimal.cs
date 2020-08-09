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
    public partial class InputDecimal: Base.UserControlInput
    {
		#region Property
		public override string Text
		{
			get { return NumericUpDown.Value.ToString(); }
			set
			{
				decimal v;
				if (decimal.TryParse(value, out v))
				{
					NumericUpDown.Value = v;
				}
				else
				{
					Console.WriteLine("数値に変換できません");
					NumericUpDown.Value = 0;

				}
			}
		}
		public decimal Maximum
		{
			get { return NumericUpDown.Maximum;  }
			set { NumericUpDown.Maximum = value; }
		}
		public decimal Minimum
		{
			get { return NumericUpDown.Minimum;  }
			set { NumericUpDown.Minimum = value; }
		}
		#endregion

		#region Constructor
		public InputDecimal()
		{
			InitializeComponent();
		}
		public InputDecimal(string text) : this()
		{
			Text = text;
		}
		#endregion

		#region Helper
		private Size GetOneCharacterSize(Font font)
		{
			//表示先
			var PictureBox = new PictureBox();
			//表示する文字列
			var s = "0";
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
			var font_size = GetOneCharacterSize(NumericUpDown.Font);

			//NumericUpDown
			var digits = 1.0;
			if (NumericUpDown.Maximum > 0)
			{
				var _max = Math.Log10((double)NumericUpDown.Maximum) + 1;
				digits = digits < _max ? _max : digits;
			}
			if (NumericUpDown.Minimum < 0)
			{
				int _min = (int)Math.Log10((double)-NumericUpDown.Minimum) + 2;
				digits = digits < _min ? _min : digits;
			}

			NumericUpDown.Size = new Size(font_size.Width * (int)digits + 20, NumericUpDown.Height);
		}
		#endregion

		#region Event
		private void numericUpDown_ValueChanged(object sender, EventArgs e)
		{

		}
		#endregion
	}
}
