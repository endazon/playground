using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Drawing;
using System.Data;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace WinFormsCtrlLibInputScreen.Base
{
	public partial class UserControlInput : UserControl
	{
		#region Property
		private uint Spacing = 0;

		public new string Name
		{
			get
			{
				((UserControl)this).Name = Label.Text;
				return ((UserControl)this).Name; 
			}
			set
			{
				((UserControl)this).Name = value;
				Label.Text = ((UserControl)this).Name;
			}
		}
		public override string Text { get; set; }
		#endregion

		#region Constructor
		public UserControlInput()
		{
			InitializeComponent();
		}
		public UserControlInput(string name) : this()
		{
			Name = name;
		}
		#endregion

		#region Helper
		private Size GetSizeAdjustmentValue(Control target)
		{
			Size _size = new Size();
			foreach (Control item in target.Controls)
			{
				Size __size;
				if (item.Controls.Count == 0) {
					__size = item.Size;
				}
				else
				{
					__size = GetSizeAdjustmentValue(item);
				}

				_size.Width += __size.Width;
				_size.Height = __size.Height < _size.Height ? _size.Height : __size.Height;
			}
			return _size;
		}

		protected virtual void UpdateDisplayBegin()
		{
			//Panel
			Panel.Size = Size;

			//UserControl
			Size = Label.PreferredSize;
		}
		protected virtual void UpdateDisplay() 
		{

		}
		protected virtual void UpdateDisplayEnd()
		{
			//Panel
			{
				var _size = GetSizeAdjustmentValue(Panel);
				Panel.Size = _size;
			}

			//UserControl
			{
				var _size = GetSizeAdjustmentValue(this);
				_size.Width += (int)Spacing;
				Size = _size;
			}
		}
		private void ExecutionUpdateDisplay()
		{
			UpdateDisplayBegin();
			UpdateDisplay();
			UpdateDisplayEnd();
		}
		#endregion

		#region PpublicMethod
		public int LabelWidth()
		{
			return Label.Width;
		}
		public void LineUp()
		{
			LineUp(0);
		}
		public void LineUp(uint span)
		{
			Spacing = span;

			ExecutionUpdateDisplay();
		}
		#endregion

		#region Event
		private void Input_Load(object sender, EventArgs e)
		{
			ExecutionUpdateDisplay();
		}
		#endregion
	}
}
