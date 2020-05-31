namespace CommandGenerator
{
	partial class FormList
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
			this.MenuStripMain = new System.Windows.Forms.MenuStrip();
			this.fileToolStripMenuItem = new System.Windows.Forms.ToolStripMenuItem();
			this.newToolStripMenuItem = new System.Windows.Forms.ToolStripMenuItem();
			this.openToolStripMenuItem = new System.Windows.Forms.ToolStripMenuItem();
			this.toolStripSeparator1 = new System.Windows.Forms.ToolStripSeparator();
			this.closeToolStripMenuItem = new System.Windows.Forms.ToolStripMenuItem();
			this.toolStripSeparator2 = new System.Windows.Forms.ToolStripSeparator();
			this.saveToolStripMenuItem = new System.Windows.Forms.ToolStripMenuItem();
			this.saveAsToolStripMenuItem = new System.Windows.Forms.ToolStripMenuItem();
			this.toolStripSeparator3 = new System.Windows.Forms.ToolStripSeparator();
			this.endToolStripMenuItem = new System.Windows.Forms.ToolStripMenuItem();
			this.setingToolStripMenuItem = new System.Windows.Forms.ToolStripMenuItem();
			this.commandListToolStripMenuItem = new System.Windows.Forms.ToolStripMenuItem();
			this.editFileOpenToolStripMenuItem = new System.Windows.Forms.ToolStripMenuItem();
			this.CommandListBox = new System.Windows.Forms.ListBox();
			this.MenuStripMain.SuspendLayout();
			this.SuspendLayout();
			// 
			// MenuStripMain
			// 
			this.MenuStripMain.BackColor = System.Drawing.SystemColors.ControlDark;
			this.MenuStripMain.Items.AddRange(new System.Windows.Forms.ToolStripItem[] {
            this.fileToolStripMenuItem,
            this.setingToolStripMenuItem,
            this.editFileOpenToolStripMenuItem});
			this.MenuStripMain.Location = new System.Drawing.Point(0, 0);
			this.MenuStripMain.Name = "MenuStripMain";
			this.MenuStripMain.Size = new System.Drawing.Size(284, 24);
			this.MenuStripMain.TabIndex = 1;
			this.MenuStripMain.Text = "MenuStripMain";
			// 
			// fileToolStripMenuItem
			// 
			this.fileToolStripMenuItem.DropDownItems.AddRange(new System.Windows.Forms.ToolStripItem[] {
            this.newToolStripMenuItem,
            this.openToolStripMenuItem,
            this.toolStripSeparator1,
            this.closeToolStripMenuItem,
            this.toolStripSeparator2,
            this.saveToolStripMenuItem,
            this.saveAsToolStripMenuItem,
            this.toolStripSeparator3,
            this.endToolStripMenuItem});
			this.fileToolStripMenuItem.Name = "fileToolStripMenuItem";
			this.fileToolStripMenuItem.Size = new System.Drawing.Size(37, 20);
			this.fileToolStripMenuItem.Text = "File";
			// 
			// newToolStripMenuItem
			// 
			this.newToolStripMenuItem.Name = "newToolStripMenuItem";
			this.newToolStripMenuItem.Size = new System.Drawing.Size(114, 22);
			this.newToolStripMenuItem.Text = "New";
			this.newToolStripMenuItem.Click += new System.EventHandler(this.newToolStripMenuItem_Click);
			// 
			// openToolStripMenuItem
			// 
			this.openToolStripMenuItem.Name = "openToolStripMenuItem";
			this.openToolStripMenuItem.Size = new System.Drawing.Size(114, 22);
			this.openToolStripMenuItem.Text = "Open";
			this.openToolStripMenuItem.Click += new System.EventHandler(this.openToolStripMenuItem_Click);
			// 
			// toolStripSeparator1
			// 
			this.toolStripSeparator1.Name = "toolStripSeparator1";
			this.toolStripSeparator1.Size = new System.Drawing.Size(111, 6);
			// 
			// closeToolStripMenuItem
			// 
			this.closeToolStripMenuItem.Name = "closeToolStripMenuItem";
			this.closeToolStripMenuItem.Size = new System.Drawing.Size(114, 22);
			this.closeToolStripMenuItem.Text = "Close";
			this.closeToolStripMenuItem.Click += new System.EventHandler(this.closeToolStripMenuItem_Click);
			// 
			// toolStripSeparator2
			// 
			this.toolStripSeparator2.Name = "toolStripSeparator2";
			this.toolStripSeparator2.Size = new System.Drawing.Size(111, 6);
			// 
			// saveToolStripMenuItem
			// 
			this.saveToolStripMenuItem.Name = "saveToolStripMenuItem";
			this.saveToolStripMenuItem.Size = new System.Drawing.Size(114, 22);
			this.saveToolStripMenuItem.Text = "Save";
			this.saveToolStripMenuItem.Click += new System.EventHandler(this.saveToolStripMenuItem_Click);
			// 
			// saveAsToolStripMenuItem
			// 
			this.saveAsToolStripMenuItem.Name = "saveAsToolStripMenuItem";
			this.saveAsToolStripMenuItem.Size = new System.Drawing.Size(114, 22);
			this.saveAsToolStripMenuItem.Text = "Save As";
			this.saveAsToolStripMenuItem.Click += new System.EventHandler(this.saveAsToolStripMenuItem_Click);
			// 
			// toolStripSeparator3
			// 
			this.toolStripSeparator3.Name = "toolStripSeparator3";
			this.toolStripSeparator3.Size = new System.Drawing.Size(111, 6);
			// 
			// endToolStripMenuItem
			// 
			this.endToolStripMenuItem.Name = "endToolStripMenuItem";
			this.endToolStripMenuItem.Size = new System.Drawing.Size(114, 22);
			this.endToolStripMenuItem.Text = "End";
			this.endToolStripMenuItem.Click += new System.EventHandler(this.endToolStripMenuItem_Click);
			// 
			// setingToolStripMenuItem
			// 
			this.setingToolStripMenuItem.DropDownItems.AddRange(new System.Windows.Forms.ToolStripItem[] {
            this.commandListToolStripMenuItem});
			this.setingToolStripMenuItem.Name = "setingToolStripMenuItem";
			this.setingToolStripMenuItem.Size = new System.Drawing.Size(52, 20);
			this.setingToolStripMenuItem.Text = "Seting";
			// 
			// commandListToolStripMenuItem
			// 
			this.commandListToolStripMenuItem.Name = "commandListToolStripMenuItem";
			this.commandListToolStripMenuItem.Size = new System.Drawing.Size(146, 22);
			this.commandListToolStripMenuItem.Text = "CommandList";
			this.commandListToolStripMenuItem.Click += new System.EventHandler(this.commandListToolStripMenuItem_Click);
			// 
			// editFileOpenToolStripMenuItem
			// 
			this.editFileOpenToolStripMenuItem.Enabled = false;
			this.editFileOpenToolStripMenuItem.Name = "editFileOpenToolStripMenuItem";
			this.editFileOpenToolStripMenuItem.Size = new System.Drawing.Size(86, 20);
			this.editFileOpenToolStripMenuItem.Text = "EditFileOpen";
			this.editFileOpenToolStripMenuItem.Click += new System.EventHandler(this.editFileOpenToolStripMenuItem_Click);
			// 
			// CommandListBox
			// 
			this.CommandListBox.BackColor = System.Drawing.SystemColors.WindowText;
			this.CommandListBox.Dock = System.Windows.Forms.DockStyle.Fill;
			this.CommandListBox.Font = new System.Drawing.Font("MS UI Gothic", 16F, System.Drawing.FontStyle.Regular, System.Drawing.GraphicsUnit.Point, ((byte)(128)));
			this.CommandListBox.ForeColor = System.Drawing.SystemColors.Window;
			this.CommandListBox.FormattingEnabled = true;
			this.CommandListBox.HorizontalScrollbar = true;
			this.CommandListBox.ItemHeight = 21;
			this.CommandListBox.Location = new System.Drawing.Point(0, 24);
			this.CommandListBox.Name = "CommandListBox";
			this.CommandListBox.Size = new System.Drawing.Size(284, 387);
			this.CommandListBox.TabIndex = 2;
			this.CommandListBox.DoubleClick += new System.EventHandler(this.CommandListBox_DoubleClick);
			this.CommandListBox.KeyDown += new System.Windows.Forms.KeyEventHandler(this.CommandListBox_KeyDown);
			// 
			// FormList
			// 
			this.AutoScaleDimensions = new System.Drawing.SizeF(6F, 12F);
			this.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font;
			this.BackColor = System.Drawing.SystemColors.ControlDarkDark;
			this.ClientSize = new System.Drawing.Size(284, 411);
			this.Controls.Add(this.CommandListBox);
			this.Controls.Add(this.MenuStripMain);
			this.MaximizeBox = false;
			this.MinimumSize = new System.Drawing.Size(300, 200);
			this.Name = "FormList";
			this.Text = "CommandList";
			this.FormClosed += new System.Windows.Forms.FormClosedEventHandler(this.FormList_FormClosed);
			this.Load += new System.EventHandler(this.FormList_Load);
			this.LocationChanged += new System.EventHandler(this.FormList_LocationChanged);
			this.SizeChanged += new System.EventHandler(this.FormList_SizeChanged);
			this.MenuStripMain.ResumeLayout(false);
			this.MenuStripMain.PerformLayout();
			this.ResumeLayout(false);
			this.PerformLayout();

		}

		#endregion

		private System.Windows.Forms.MenuStrip MenuStripMain;
		private System.Windows.Forms.ToolStripMenuItem fileToolStripMenuItem;
		private System.Windows.Forms.ToolStripMenuItem newToolStripMenuItem;
		private System.Windows.Forms.ToolStripMenuItem openToolStripMenuItem;
		private System.Windows.Forms.ToolStripSeparator toolStripSeparator1;
		private System.Windows.Forms.ToolStripMenuItem closeToolStripMenuItem;
		private System.Windows.Forms.ToolStripSeparator toolStripSeparator2;
		private System.Windows.Forms.ToolStripMenuItem saveToolStripMenuItem;
		private System.Windows.Forms.ToolStripMenuItem saveAsToolStripMenuItem;
		private System.Windows.Forms.ToolStripSeparator toolStripSeparator3;
		private System.Windows.Forms.ToolStripMenuItem endToolStripMenuItem;
		private System.Windows.Forms.ListBox CommandListBox;
		private System.Windows.Forms.ToolStripMenuItem editFileOpenToolStripMenuItem;
		private System.Windows.Forms.ToolStripMenuItem setingToolStripMenuItem;
		private System.Windows.Forms.ToolStripMenuItem commandListToolStripMenuItem;
	}
}