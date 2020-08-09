namespace CommandGenerator
{
	partial class FormCommunicationDsplay
	{
		/// <summary>
		/// Required designer variable.
		/// </summary>
		private System.ComponentModel.IContainer components = null;

		/// <summary>
		/// Clean up any resources being used.
		/// </summary>
		/// <param name="disposing">true if managed resources should be disposed; otherwise, false.</param>
		protected override void Dispose(bool disposing)
		{
			if (disposing && (components != null))
			{
				components.Dispose();
			}
			base.Dispose(disposing);
		}

		#region Windows Form Designer generated code

		/// <summary>
		/// Required method for Designer support - do not modify
		/// the contents of this method with the code editor.
		/// </summary>
		private void InitializeComponent()
		{
			this.listBoxComData = new System.Windows.Forms.ListBox();
			this.SuspendLayout();
			// 
			// listBoxComData
			// 
			this.listBoxComData.BackColor = System.Drawing.SystemColors.ControlDarkDark;
			this.listBoxComData.Dock = System.Windows.Forms.DockStyle.Fill;
			this.listBoxComData.ForeColor = System.Drawing.SystemColors.Control;
			this.listBoxComData.FormattingEnabled = true;
			this.listBoxComData.HorizontalScrollbar = true;
			this.listBoxComData.ItemHeight = 12;
			this.listBoxComData.Location = new System.Drawing.Point(0, 0);
			this.listBoxComData.Name = "listBoxComData";
			this.listBoxComData.Size = new System.Drawing.Size(884, 411);
			this.listBoxComData.TabIndex = 0;
			// 
			// FormCommunicationDsplay
			// 
			this.AutoScaleDimensions = new System.Drawing.SizeF(6F, 12F);
			this.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font;
			this.AutoScroll = true;
			this.BackColor = System.Drawing.SystemColors.ControlDarkDark;
			this.ClientSize = new System.Drawing.Size(884, 411);
			this.Controls.Add(this.listBoxComData);
			this.ForeColor = System.Drawing.SystemColors.Window;
			this.MinimumSize = new System.Drawing.Size(300, 100);
			this.Name = "FormCommunicationDsplay";
			this.Text = "FormCommunicationDsplay";
			this.ResumeLayout(false);

		}

		#endregion

		private System.Windows.Forms.ListBox listBoxComData;
	}
}