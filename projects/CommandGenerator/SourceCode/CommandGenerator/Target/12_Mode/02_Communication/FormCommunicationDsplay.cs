using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Data;
using System.Drawing;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace CommandGenerator.Target._12_Mode._02_Communication
{
	public partial class FormCommunicationDsplay : FormEdit
	{
		public FormCommunicationDsplay()
		{
			InitializeComponent();

			chgListBoxFont(new Font("MS UI Gothic", 8F, FontStyle.Regular, GraphicsUnit.Point, ((byte)(128))));
		}

		public override void LocationUpdate(int x, int y)
		{
			int buff_x = x >= 0 ? x : Location.X;
			int buff_y = y >= 0 ? y : Location.Y;

			Location = new Point(buff_x - 5, buff_y);
		}

		public override void SizeUpdate(int width, int height)
		{
			int buff_width = width >= 0 ? width : Size.Width;
			int buff_height = height >= 0 ? height : Size.Height;

			Size = new Size(buff_width, buff_height);
		}

		public void WriteLine(string str)
		{
			Add(str);
		}
	}
}
