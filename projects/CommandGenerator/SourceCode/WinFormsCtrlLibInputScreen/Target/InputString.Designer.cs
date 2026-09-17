using System;
using System.Drawing;

namespace WinFormsCtrlLibInputScreen
{
	partial class InputString
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
			this.TextBox = new System.Windows.Forms.TextBox();
			this.SuspendLayout();
			// 
			// Panel
			// 
			this.Panel.Controls.Add(this.TextBox);
			this.Panel.Location = new System.Drawing.Point(32, 0);
			this.Panel.Size = new System.Drawing.Size(68, 12);
			// 
			// TextBox
			// 
			this.TextBox.AllowDrop = true;
			this.TextBox.BackColor = System.Drawing.SystemColors.WindowText;
			this.TextBox.BorderStyle = System.Windows.Forms.BorderStyle.None;
			this.TextBox.Dock = System.Windows.Forms.DockStyle.Left;
			this.TextBox.ForeColor = System.Drawing.SystemColors.Window;
			this.TextBox.Location = new System.Drawing.Point(0, 0);
			this.TextBox.Margin = new System.Windows.Forms.Padding(0);
			this.TextBox.MaxLength = 10;
			this.TextBox.Name = "TextBox";
			this.TextBox.Size = new System.Drawing.Size(68, 12);
			this.TextBox.TabIndex = 2;
			this.TextBox.Text = "TextBox";
			this.TextBox.TextChanged += new System.EventHandler(this.textBox_TextChanged);
			this.TextBox.DragDrop += new System.Windows.Forms.DragEventHandler(this.TextBox_DragDrop);
			this.TextBox.DragEnter += new System.Windows.Forms.DragEventHandler(this.TextBox_DragEnter);
			// 
			// InputString
			// 
			this.AutoScaleDimensions = new System.Drawing.SizeF(6F, 12F);
			this.Name = "InputString";
			this.Size = new System.Drawing.Size(100, 12);
			this.ResumeLayout(false);
			this.PerformLayout();

		}

		#endregion
		private System.Windows.Forms.TextBox TextBox;
	}
}
