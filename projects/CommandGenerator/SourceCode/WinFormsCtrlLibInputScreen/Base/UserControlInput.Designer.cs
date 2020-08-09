namespace WinFormsCtrlLibInputScreen.Base
{
	partial class UserControlInput
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
			this.Label = new System.Windows.Forms.Label();
			this.Panel = new System.Windows.Forms.Panel();
			this.SuspendLayout();
			// 
			// Label
			// 
			this.Label.AutoSize = true;
			this.Label.Dock = System.Windows.Forms.DockStyle.Left;
			this.Label.FlatStyle = System.Windows.Forms.FlatStyle.Flat;
			this.Label.ForeColor = System.Drawing.SystemColors.Window;
			this.Label.Location = new System.Drawing.Point(0, 0);
			this.Label.Margin = new System.Windows.Forms.Padding(0);
			this.Label.Name = "Label";
			this.Label.Size = new System.Drawing.Size(32, 12);
			this.Label.TabIndex = 1;
			this.Label.Text = "Label";
			// 
			// Panel
			// 
			this.Panel.BackColor = System.Drawing.SystemColors.ControlDarkDark;
			this.Panel.Dock = System.Windows.Forms.DockStyle.Right;
			this.Panel.Location = new System.Drawing.Point(46, 0);
			this.Panel.Margin = new System.Windows.Forms.Padding(0);
			this.Panel.Name = "Panel";
			this.Panel.Size = new System.Drawing.Size(144, 20);
			this.Panel.TabIndex = 2;
			this.Panel.Text = "Panel";
			// 
			// UserControlInput
			// 
			this.AutoScaleDimensions = new System.Drawing.SizeF(6F, 12F);
			this.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font;
			this.BackColor = System.Drawing.SystemColors.ControlDarkDark;
			this.Controls.Add(this.Panel);
			this.Controls.Add(this.Label);
			this.Name = "UserControlInput";
			this.Size = new System.Drawing.Size(190, 20);
			this.Load += new System.EventHandler(this.Input_Load);
			this.ResumeLayout(false);
			this.PerformLayout();

		}

		#endregion
		protected System.Windows.Forms.Label Label;
		protected System.Windows.Forms.Panel Panel;
	}
}
