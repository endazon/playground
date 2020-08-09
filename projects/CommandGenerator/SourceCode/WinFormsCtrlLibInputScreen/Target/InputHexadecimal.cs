using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Data;
using System.Drawing;
using System.Text;
using System.Windows.Forms;

namespace WinFormsCtrlLibInputScreen
{
	public partial class InputHexadecimal : WinFormsCtrlLibInputScreen.InputString
	{
		private static readonly string HEADER_STR = "0x";
		private static readonly int    MIN_LENGTH = 2;

		#region Property
		public override string Text
		{
			get { return base.Text;               }
			set { UpdateText(value, value.Length); }
		}
		public override int MaxLength
		{
			get { return base.MaxLength;               }
			set { base.MaxLength = value + MIN_LENGTH; }
		}
		#endregion

		#region Constructor
		public InputHexadecimal()
		{
			InitializeComponent();
		}
		public InputHexadecimal(string name) : this()
		{
			Name = name;
		}
		#endregion

		#region Helper
		private bool UpdateText(string txt, int len)
		{
			//フォーマット通りの文字列かチェック
			//NGであれば初期化する
			if ((len < MIN_LENGTH)
				|| txt.Substring(0, 1) != HEADER_STR.Substring(0, 1)
				|| txt.Substring(1, 1) != HEADER_STR.Substring(1, 1))
			{
				base.Text = HEADER_STR;
				return false;
			}

			//初期値であれば何もしない
			//※念のため初期値をセット
			else if (len == MIN_LENGTH) {
				base.Text = HEADER_STR;
				return true;
			}

			//入力文字列の先頭からが16進数表記かチェックする
			//NGであればNGになるまでの文字列をセットする
			else
			{
				var char_pos = MIN_LENGTH;
				foreach (var c in txt.Substring(char_pos, len - char_pos))
				{
					if (!Uri.IsHexDigit(c))
					{
						base.Text = txt.Remove(char_pos, 1);
						return false;
					}
					char_pos++;
				}
			}

			return true;
		}
		#endregion

		#region Event
		protected override void textBox_TextChanged(object sender, EventArgs e)
		{
			TextBox target = (TextBox)sender;

			if (!UpdateText((string)target.Text.Clone(), target.Text.Length))
			{
				System.Media.SystemSounds.Asterisk.Play();
			}
			target.SelectionStart = target.TextLength;
		}
		#endregion
	}
}
