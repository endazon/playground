namespace WinFormsCtrlLibInputScreen
{
	partial class InputFile
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
			this.Button = new System.Windows.Forms.Button();
			this.SuspendLayout();
			// 
			// Panel
			// 
			this.Panel.Controls.Add(this.Button);
			this.Panel.Size = new System.Drawing.Size(100, 56);
			this.Panel.Controls.SetChildIndex(this.Button, 0);
			// 
			// Button
			// 
			this.Button.BackColor = System.Drawing.SystemColors.ButtonShadow;
			this.Button.Dock = System.Windows.Forms.DockStyle.Left;
			this.Button.FlatStyle = System.Windows.Forms.FlatStyle.Flat;
			this.Button.ForeColor = System.Drawing.SystemColors.ControlText;
			this.Button.Location = new System.Drawing.Point(80, 0);
			this.Button.Name = "Button";
			this.Button.Size = new System.Drawing.Size(20, 56);
			this.Button.TabIndex = 3;
			this.Button.Text = ".....";
			this.Button.UseVisualStyleBackColor = false;
			this.Button.Click += new System.EventHandler(this.Button_Click);
			// 
			// InputFile
			// 
			this.AutoScaleDimensions = new System.Drawing.SizeF(6F, 12F);
			this.Name = "InputFile";
			this.Size = new System.Drawing.Size(132, 56);
			this.ResumeLayout(false);
			this.PerformLayout();

		}

		#endregion

		private System.Windows.Forms.Button Button;
	}
}
