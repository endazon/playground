using System;
using System.Drawing;
using System.IO;
using System.Security.Permissions;
using System.Windows.Forms;

namespace CommandGenerator
{
	public partial class FormGenerate : FormEdit
	{
		public FormGenerate()
		{
			InitializeComponent();
		}
		public FormGenerate(string name, string version)
			: base(name, version)
		{
			InitializeComponent();
		}

		#region Button
		public override void buttonGenerates_Click(object sender, EventArgs e)
		{
			Save();
		}
		#endregion

		#region Window
		public override void FormEdit_FormClosed(object sender, FormClosedEventArgs e)
		{
			var delete_path = Environment.CurrentDirectory;

			foreach (var file in Directory.GetFiles(delete_path))
			{
				if (Path.GetFileName(file).Contains("FileData"))
				{
					File.Delete(@file);
				}
			}
		}
		#endregion
	}
}
