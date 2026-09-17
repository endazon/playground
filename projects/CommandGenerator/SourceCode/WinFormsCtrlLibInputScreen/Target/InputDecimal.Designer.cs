using System;
using System.Drawing;

namespace WinFormsCtrlLibInputScreen
{
    partial class InputDecimal
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
			this.NumericUpDown = new System.Windows.Forms.NumericUpDown();
			((System.ComponentModel.ISupportInitialize)(this.NumericUpDown)).BeginInit();
			this.SuspendLayout();
			// 
			// Panel
			// 
			this.Panel.Controls.Add(this.NumericUpDown);
			this.Panel.Location = new System.Drawing.Point(32, 0);
			this.Panel.Size = new System.Drawing.Size(68, 15);
			// 
			// NumericUpDown
			// 
			this.NumericUpDown.BackColor = System.Drawing.SystemColors.WindowText;
			this.NumericUpDown.BorderStyle = System.Windows.Forms.BorderStyle.None;
			this.NumericUpDown.Dock = System.Windows.Forms.DockStyle.Left;
			this.NumericUpDown.ForeColor = System.Drawing.SystemColors.Window;
			this.NumericUpDown.Location = new System.Drawing.Point(0, 0);
			this.NumericUpDown.Margin = new System.Windows.Forms.Padding(0);
			this.NumericUpDown.Minimum = new decimal(new int[] {
            100,
            0,
            0,
            -2147483648});
			this.NumericUpDown.Name = "NumericUpDown";
			this.NumericUpDown.Size = new System.Drawing.Size(68, 15);
			this.NumericUpDown.TabIndex = 1;
			this.NumericUpDown.ValueChanged += new System.EventHandler(this.numericUpDown_ValueChanged);
			// 
			// InputDecimal
			// 
			this.AutoScaleDimensions = new System.Drawing.SizeF(6F, 12F);
			this.Name = "InputDecimal";
			this.Size = new System.Drawing.Size(100, 15);
			((System.ComponentModel.ISupportInitialize)(this.NumericUpDown)).EndInit();
			this.ResumeLayout(false);
			this.PerformLayout();

        }

        #endregion

        private System.Windows.Forms.NumericUpDown NumericUpDown;
    }
}
