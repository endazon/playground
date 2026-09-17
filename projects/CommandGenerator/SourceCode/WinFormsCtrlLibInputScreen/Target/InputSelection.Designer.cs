namespace WinFormsCtrlLibInputScreen
{
	partial class InputSelection
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

		#region コンポーネント デザイナーで生成されたコード

		/// <summary> 
		/// デザイナー サポートに必要なメソッドです。このメソッドの内容を 
		/// コード エディターで変更しないでください。
		/// </summary>
		private void InitializeComponent()
		{
			this.ComboBox = new System.Windows.Forms.ComboBox();
			this.Panel.SuspendLayout();
			this.SuspendLayout();
			// 
			// Panel
			// 
			this.Panel.Controls.Add(this.ComboBox);
			this.Panel.Location = new System.Drawing.Point(32, 0);
			this.Panel.Size = new System.Drawing.Size(91, 20);
			// 
			// ComboBox
			// 
			this.ComboBox.BackColor = System.Drawing.SystemColors.WindowText;
			this.ComboBox.Dock = System.Windows.Forms.DockStyle.Left;
			this.ComboBox.FlatStyle = System.Windows.Forms.FlatStyle.Flat;
			this.ComboBox.ForeColor = System.Drawing.SystemColors.Window;
			this.ComboBox.FormattingEnabled = true;
			this.ComboBox.Location = new System.Drawing.Point(0, 0);
			this.ComboBox.Margin = new System.Windows.Forms.Padding(0);
			this.ComboBox.Name = "ComboBox";
			this.ComboBox.Size = new System.Drawing.Size(91, 20);
			this.ComboBox.TabIndex = 2;
			this.ComboBox.Text = "ComboBox";
			// 
			// InputSelection
			// 
			this.AutoScaleDimensions = new System.Drawing.SizeF(6F, 12F);
			this.Name = "InputSelection";
			this.Size = new System.Drawing.Size(123, 20);
			this.Panel.ResumeLayout(false);
			this.ResumeLayout(false);
			this.PerformLayout();

		}

		#endregion

		private System.Windows.Forms.ComboBox ComboBox;
	}
}
