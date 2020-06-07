namespace CommandGenerator
{
	partial class FormEdit
	{
		/// <summary>
		/// 必要なデザイナー変数です。
		/// </summary>
		private System.ComponentModel.IContainer components = null;

		/// <summary>
		/// 使用中のリソースをすべてクリーンアップします。
		/// </summary>
		/// <param name="disposing">マネージド リソースを破棄する場合は true を指定し、その他の場合は false を指定します。</param>
		protected override void Dispose(bool disposing)
		{
			if (disposing && (components != null))
			{
				components.Dispose();
			}
			base.Dispose(disposing);
		}

		#region Windows フォーム デザイナーで生成されたコード

		/// <summary>
		/// デザイナー サポートに必要なメソッドです。このメソッドの内容を
		/// コード エディターで変更しないでください。
		/// </summary>
		private void InitializeComponent()
		{
			this.MenuStripMain = new System.Windows.Forms.MenuStrip();
			this.fileToolStripMenuItem = new System.Windows.Forms.ToolStripMenuItem();
			this.newToolStripMenuItem = new System.Windows.Forms.ToolStripMenuItem();
			this.toolStripSeparator2 = new System.Windows.Forms.ToolStripSeparator();
			this.saveToolStripMenuItem = new System.Windows.Forms.ToolStripMenuItem();
			this.saveAsToolStripMenuItem = new System.Windows.Forms.ToolStripMenuItem();
			this.SplitContainer = new System.Windows.Forms.SplitContainer();
			this.buttonGenerates = new System.Windows.Forms.Button();
			this.CommandListBox = new System.Windows.Forms.ListBox();
			this.saveFileDialogCsv = new System.Windows.Forms.SaveFileDialog();
			this.MenuStripMain.SuspendLayout();
			((System.ComponentModel.ISupportInitialize)(this.SplitContainer)).BeginInit();
			this.SplitContainer.Panel1.SuspendLayout();
			this.SplitContainer.SuspendLayout();
			this.SuspendLayout();
			// 
			// MenuStripMain
			// 
			this.MenuStripMain.BackColor = System.Drawing.SystemColors.ControlDark;
			this.MenuStripMain.Items.AddRange(new System.Windows.Forms.ToolStripItem[] {
            this.fileToolStripMenuItem});
			this.MenuStripMain.Location = new System.Drawing.Point(0, 0);
			this.MenuStripMain.Name = "MenuStripMain";
			this.MenuStripMain.Size = new System.Drawing.Size(884, 24);
			this.MenuStripMain.TabIndex = 0;
			this.MenuStripMain.Text = "MenuStripMain";
			// 
			// fileToolStripMenuItem
			// 
			this.fileToolStripMenuItem.DropDownItems.AddRange(new System.Windows.Forms.ToolStripItem[] {
            this.newToolStripMenuItem,
            this.toolStripSeparator2,
            this.saveToolStripMenuItem,
            this.saveAsToolStripMenuItem});
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
			// SplitContainer
			// 
			this.SplitContainer.BackColor = System.Drawing.SystemColors.Control;
			this.SplitContainer.BorderStyle = System.Windows.Forms.BorderStyle.Fixed3D;
			this.SplitContainer.Dock = System.Windows.Forms.DockStyle.Fill;
			this.SplitContainer.FixedPanel = System.Windows.Forms.FixedPanel.Panel1;
			this.SplitContainer.Location = new System.Drawing.Point(0, 24);
			this.SplitContainer.Name = "SplitContainer";
			// 
			// SplitContainer.Panel1
			// 
			this.SplitContainer.Panel1.BackColor = System.Drawing.SystemColors.ControlDarkDark;
			this.SplitContainer.Panel1.Controls.Add(this.buttonGenerates);
			this.SplitContainer.Panel1.Controls.Add(this.CommandListBox);
			// 
			// SplitContainer.Panel2
			// 
			this.SplitContainer.Panel2.AutoScroll = true;
			this.SplitContainer.Panel2.BackColor = System.Drawing.SystemColors.ControlDarkDark;
			this.SplitContainer.Size = new System.Drawing.Size(884, 387);
			this.SplitContainer.SplitterDistance = 215;
			this.SplitContainer.TabIndex = 1;
			// 
			// buttonGenerates
			// 
			this.buttonGenerates.BackColor = System.Drawing.SystemColors.Control;
			this.buttonGenerates.Dock = System.Windows.Forms.DockStyle.Bottom;
			this.buttonGenerates.ForeColor = System.Drawing.SystemColors.ControlText;
			this.buttonGenerates.Location = new System.Drawing.Point(0, 360);
			this.buttonGenerates.Name = "buttonGenerates";
			this.buttonGenerates.Size = new System.Drawing.Size(211, 23);
			this.buttonGenerates.TabIndex = 0;
			this.buttonGenerates.Text = "Generates";
			this.buttonGenerates.UseVisualStyleBackColor = false;
			this.buttonGenerates.Click += new System.EventHandler(this.buttonGenerates_Click);
			// 
			// CommandListBox
			// 
			this.CommandListBox.Anchor = ((System.Windows.Forms.AnchorStyles)((((System.Windows.Forms.AnchorStyles.Top | System.Windows.Forms.AnchorStyles.Bottom) 
            | System.Windows.Forms.AnchorStyles.Left) 
            | System.Windows.Forms.AnchorStyles.Right)));
			this.CommandListBox.BackColor = System.Drawing.SystemColors.WindowText;
			this.CommandListBox.Font = new System.Drawing.Font("MS UI Gothic", 16F, System.Drawing.FontStyle.Regular, System.Drawing.GraphicsUnit.Point, ((byte)(128)));
			this.CommandListBox.ForeColor = System.Drawing.SystemColors.Window;
			this.CommandListBox.FormattingEnabled = true;
			this.CommandListBox.HorizontalScrollbar = true;
			this.CommandListBox.ItemHeight = 21;
			this.CommandListBox.Location = new System.Drawing.Point(0, 0);
			this.CommandListBox.Name = "CommandListBox";
			this.CommandListBox.Size = new System.Drawing.Size(211, 361);
			this.CommandListBox.TabIndex = 0;
			this.CommandListBox.SelectedIndexChanged += new System.EventHandler(this.CommandListBox_SelectedIndexChanged);
			this.CommandListBox.DoubleClick += new System.EventHandler(this.CommandListBox_DoubleClick);
			// 
			// saveFileDialogCsv
			// 
			this.saveFileDialogCsv.FileName = "default.csv";
			this.saveFileDialogCsv.Filter = "CSVファイル(*.csv)|*.csv";
			this.saveFileDialogCsv.RestoreDirectory = true;
			this.saveFileDialogCsv.Title = "保存先を指定してください";
			// 
			// FormEdit
			// 
			this.AutoScaleDimensions = new System.Drawing.SizeF(6F, 12F);
			this.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font;
			this.BackColor = System.Drawing.SystemColors.ControlDarkDark;
			this.ClientSize = new System.Drawing.Size(884, 411);
			this.Controls.Add(this.SplitContainer);
			this.Controls.Add(this.MenuStripMain);
			this.ForeColor = System.Drawing.SystemColors.ControlLight;
			this.MainMenuStrip = this.MenuStripMain;
			this.MaximizeBox = false;
			this.MinimumSize = new System.Drawing.Size(300, 450);
			this.Name = "FormEdit";
			this.Text = "CommandEdit";
			this.FormClosed += new System.Windows.Forms.FormClosedEventHandler(this.FormEdit_FormClosed);
			this.Load += new System.EventHandler(this.FormEdit_Load);
			this.MenuStripMain.ResumeLayout(false);
			this.MenuStripMain.PerformLayout();
			this.SplitContainer.Panel1.ResumeLayout(false);
			((System.ComponentModel.ISupportInitialize)(this.SplitContainer)).EndInit();
			this.SplitContainer.ResumeLayout(false);
			this.ResumeLayout(false);
			this.PerformLayout();

		}

		#endregion

		private System.Windows.Forms.MenuStrip MenuStripMain;
		private System.Windows.Forms.SplitContainer SplitContainer;
		private System.Windows.Forms.ToolStripMenuItem fileToolStripMenuItem;
		private System.Windows.Forms.ToolStripMenuItem newToolStripMenuItem;
		private System.Windows.Forms.ToolStripSeparator toolStripSeparator2;
		private System.Windows.Forms.ToolStripMenuItem saveToolStripMenuItem;
		private System.Windows.Forms.ToolStripMenuItem saveAsToolStripMenuItem;
		private System.Windows.Forms.ListBox CommandListBox;
		private System.Windows.Forms.Button buttonGenerates;
		private System.Windows.Forms.SaveFileDialog saveFileDialogCsv;
	}
}

