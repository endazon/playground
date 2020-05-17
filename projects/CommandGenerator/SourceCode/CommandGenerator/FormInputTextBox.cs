using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Data;
using System.Drawing;
using System.Linq;
using System.Security.Permissions;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace CommandGenerator
{
	public partial class FormInputTextBox : Form
	{
		private string _GetInputText { get; set; } = "";
		public FormInputTextBox()
		{
			InitializeComponent();
			_GetInputText = "";
		}
		public FormInputTextBox(string display)
			: this()
		{
			label.Text   = display;
		}
		public FormInputTextBox(string display, string title)
			: this(display)
		{
			Text         = title;
		}
		public FormInputTextBox(string display, string title, string _default)
			: this(display, title)
		{
			textBox.Text = _default;
		}
		public FormInputTextBox(string display, string title, string _default, Point point)
			:this(display, title, _default)
		{
			Location     = point;
		}
		public FormInputTextBox(string display, string title, string _default, int x, int y)
			: this(display, title, _default)
		{
			Location     = new Point(x, y);
		}

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

		public string GetInputText()
		{
			ShowDialog();
			return (string)_GetInputText.Clone();
		}

		private void buttonOK_Click(object sender, EventArgs e)
		{
			_GetInputText = textBox.Text;
			Close();
		}

		private void buttonCancel_Click(object sender, EventArgs e)
		{
			_GetInputText = "";
			Close();
		}
	}
}
