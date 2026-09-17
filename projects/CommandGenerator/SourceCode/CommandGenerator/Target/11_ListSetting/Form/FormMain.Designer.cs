namespace ListSettings
{
	partial class FormMain
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
			this.components = new System.ComponentModel.Container();
			this.splitContainer = new System.Windows.Forms.SplitContainer();
			this.button = new System.Windows.Forms.Button();
			this.treeView = new System.Windows.Forms.TreeView();
			this.contextMenuStrip = new System.Windows.Forms.ContextMenuStrip(this.components);
			this.addToolStripMenuItem = new System.Windows.Forms.ToolStripMenuItem();
			this.removeToolStripMenuItem = new System.Windows.Forms.ToolStripMenuItem();
			this.textBoxVersion = new System.Windows.Forms.TextBox();
			this.textBoxName = new System.Windows.Forms.TextBox();
			this.panel1 = new System.Windows.Forms.Panel();
			this.panel2 = new System.Windows.Forms.Panel();
			((System.ComponentModel.ISupportInitialize)(this.splitContainer)).BeginInit();
			this.splitContainer.Panel1.SuspendLayout();
			this.splitContainer.SuspendLayout();
			this.contextMenuStrip.SuspendLayout();
			this.panel1.SuspendLayout();
			this.panel2.SuspendLayout();
			this.SuspendLayout();
			// 
			// splitContainer
			// 
			this.splitContainer.BackColor = System.Drawing.SystemColors.Control;
			this.splitContainer.BorderStyle = System.Windows.Forms.BorderStyle.Fixed3D;
			this.splitContainer.Dock = System.Windows.Forms.DockStyle.Fill;
			this.splitContainer.FixedPanel = System.Windows.Forms.FixedPanel.Panel1;
			this.splitContainer.Location = new System.Drawing.Point(0, 0);
			this.splitContainer.Name = "splitContainer";
			// 
			// splitContainer.Panel1
			// 
			this.splitContainer.Panel1.BackColor = System.Drawing.SystemColors.ControlDarkDark;
			this.splitContainer.Panel1.Controls.Add(this.button);
			this.splitContainer.Panel1.Controls.Add(this.treeView);
			// 
			// splitContainer.Panel2
			// 
			this.splitContainer.Panel2.AutoScroll = true;
			this.splitContainer.Panel2.BackColor = System.Drawing.SystemColors.ControlDarkDark;
			this.splitContainer.Size = new System.Drawing.Size(634, 398);
			this.splitContainer.SplitterDistance = 210;
			this.splitContainer.TabIndex = 3;
			// 
			// button
			// 
			this.button.BackColor = System.Drawing.SystemColors.Control;
			this.button.Dock = System.Windows.Forms.DockStyle.Bottom;
			this.button.ForeColor = System.Drawing.SystemColors.ControlText;
			this.button.Location = new System.Drawing.Point(0, 371);
			this.button.Name = "button";
			this.button.Size = new System.Drawing.Size(206, 23);
			this.button.TabIndex = 3;
			this.button.Text = "Create";
			this.button.UseVisualStyleBackColor = false;
			this.button.Click += new System.EventHandler(this.button_Click);
			// 
			// treeView
			// 
			this.treeView.Anchor = ((System.Windows.Forms.AnchorStyles)((((System.Windows.Forms.AnchorStyles.Top | System.Windows.Forms.AnchorStyles.Bottom) 
            | System.Windows.Forms.AnchorStyles.Left) 
            | System.Windows.Forms.AnchorStyles.Right)));
			this.treeView.BackColor = System.Drawing.SystemColors.WindowText;
			this.treeView.ContextMenuStrip = this.contextMenuStrip;
			this.treeView.Font = new System.Drawing.Font("MS UI Gothic", 16F, System.Drawing.FontStyle.Regular, System.Drawing.GraphicsUnit.Point, ((byte)(128)));
			this.treeView.ForeColor = System.Drawing.SystemColors.Window;
			this.treeView.LabelEdit = true;
			this.treeView.Location = new System.Drawing.Point(0, 0);
			this.treeView.Name = "treeView";
			this.treeView.Size = new System.Drawing.Size(206, 370);
			this.treeView.TabIndex = 2;
			this.treeView.AfterSelect += new System.Windows.Forms.TreeViewEventHandler(this.treeView_AfterSelect);
			// 
			// contextMenuStrip
			// 
			this.contextMenuStrip.Items.AddRange(new System.Windows.Forms.ToolStripItem[] {
            this.addToolStripMenuItem,
            this.removeToolStripMenuItem});
			this.contextMenuStrip.Name = "contextMenuStrip";
			this.contextMenuStrip.Size = new System.Drawing.Size(117, 48);
			// 
			// addToolStripMenuItem
			// 
			this.addToolStripMenuItem.Name = "addToolStripMenuItem";
			this.addToolStripMenuItem.Size = new System.Drawing.Size(116, 22);
			this.addToolStripMenuItem.Text = "Add";
			this.addToolStripMenuItem.Click += new System.EventHandler(this.addToolStripMenuItem_Click);
			// 
			// removeToolStripMenuItem
			// 
			this.removeToolStripMenuItem.Name = "removeToolStripMenuItem";
			this.removeToolStripMenuItem.Size = new System.Drawing.Size(116, 22);
			this.removeToolStripMenuItem.Text = "Remove";
			this.removeToolStripMenuItem.Click += new System.EventHandler(this.removeToolStripMenuItem_Click);
			// 
			// textBoxVersion
			// 
			this.textBoxVersion.BackColor = System.Drawing.SystemColors.WindowText;
			this.textBoxVersion.Dock = System.Windows.Forms.DockStyle.Top;
			this.textBoxVersion.Font = new System.Drawing.Font("MS UI Gothic", 16F, System.Drawing.FontStyle.Regular, System.Drawing.GraphicsUnit.Point, ((byte)(128)));
			this.textBoxVersion.ForeColor = System.Drawing.SystemColors.Window;
			this.textBoxVersion.Location = new System.Drawing.Point(0, 29);
			this.textBoxVersion.Name = "textBoxVersion";
			this.textBoxVersion.Size = new System.Drawing.Size(630, 29);
			this.textBoxVersion.TabIndex = 1;
			// 
			// textBoxName
			// 
			this.textBoxName.BackColor = System.Drawing.SystemColors.WindowText;
			this.textBoxName.Dock = System.Windows.Forms.DockStyle.Top;
			this.textBoxName.Font = new System.Drawing.Font("MS UI Gothic", 16F, System.Drawing.FontStyle.Regular, System.Drawing.GraphicsUnit.Point, ((byte)(128)));
			this.textBoxName.ForeColor = System.Drawing.SystemColors.Window;
			this.textBoxName.Location = new System.Drawing.Point(0, 0);
			this.textBoxName.Name = "textBoxName";
			this.textBoxName.Size = new System.Drawing.Size(630, 29);
			this.textBoxName.TabIndex = 0;
			// 
			// panel1
			// 
			this.panel1.BorderStyle = System.Windows.Forms.BorderStyle.Fixed3D;
			this.panel1.Controls.Add(this.textBoxVersion);
			this.panel1.Controls.Add(this.textBoxName);
			this.panel1.Dock = System.Windows.Forms.DockStyle.Top;
			this.panel1.Location = new System.Drawing.Point(0, 0);
			this.panel1.Name = "panel1";
			this.panel1.Size = new System.Drawing.Size(634, 63);
			this.panel1.TabIndex = 1;
			// 
			// panel2
			// 
			this.panel2.Controls.Add(this.splitContainer);
			this.panel2.Dock = System.Windows.Forms.DockStyle.Fill;
			this.panel2.Location = new System.Drawing.Point(0, 63);
			this.panel2.Name = "panel2";
			this.panel2.Size = new System.Drawing.Size(634, 398);
			this.panel2.TabIndex = 0;
			// 
			// FormMain
			// 
			this.AutoScaleDimensions = new System.Drawing.SizeF(6F, 12F);
			this.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font;
			this.BackColor = System.Drawing.SystemColors.ControlDarkDark;
			this.ClientSize = new System.Drawing.Size(634, 461);
			this.Controls.Add(this.panel2);
			this.Controls.Add(this.panel1);
			this.ForeColor = System.Drawing.SystemColors.ControlLight;
			this.MinimumSize = new System.Drawing.Size(650, 500);
			this.Name = "FormMain";
			this.Text = "ListSetting";
			this.FormClosed += new System.Windows.Forms.FormClosedEventHandler(this.FormListEditor_FormClosed);
			this.Shown += new System.EventHandler(this.FormListEditor_Shown);
			this.splitContainer.Panel1.ResumeLayout(false);
			((System.ComponentModel.ISupportInitialize)(this.splitContainer)).EndInit();
			this.splitContainer.ResumeLayout(false);
			this.contextMenuStrip.ResumeLayout(false);
			this.panel1.ResumeLayout(false);
			this.panel1.PerformLayout();
			this.panel2.ResumeLayout(false);
			this.ResumeLayout(false);

		}

		#endregion

		private System.Windows.Forms.SplitContainer splitContainer;
		private System.Windows.Forms.TreeView treeView;
		private System.Windows.Forms.TextBox textBoxVersion;
		private System.Windows.Forms.TextBox textBoxName;
		private System.Windows.Forms.Panel panel1;
		private System.Windows.Forms.Panel panel2;
		private System.Windows.Forms.ContextMenuStrip contextMenuStrip;
		private System.Windows.Forms.ToolStripMenuItem addToolStripMenuItem;
		private System.Windows.Forms.ToolStripMenuItem removeToolStripMenuItem;
		private System.Windows.Forms.Button button;
	}
}