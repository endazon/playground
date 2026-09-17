namespace TestWindowsFormsApp
{
	partial class Form1
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
			this.inputString1 = new WinFormsCtrlLibInputScreen.InputString();
			this.inputSelection1 = new WinFormsCtrlLibInputScreen.InputSelection();
			this.inputHexadecimal1 = new WinFormsCtrlLibInputScreen.InputHexadecimal();
			this.inputFile1 = new WinFormsCtrlLibInputScreen.InputFile();
			this.inputDecimal1 = new WinFormsCtrlLibInputScreen.InputDecimal();
			this.SuspendLayout();
			// 
			// inputString1
			// 
			this.inputString1.BackColor = System.Drawing.SystemColors.ControlDarkDark;
			this.inputString1.Font = new System.Drawing.Font("MS UI Gothic", 16F, System.Drawing.FontStyle.Regular, System.Drawing.GraphicsUnit.Point, ((byte)(128)));
			this.inputString1.Location = new System.Drawing.Point(79, 253);
			this.inputString1.Margin = new System.Windows.Forms.Padding(6, 5, 6, 5);
			this.inputString1.MaxLength = 10;
			this.inputString1.Name = "inputString1";
			this.inputString1.Size = new System.Drawing.Size(199, 22);
			this.inputString1.TabIndex = 4;
			this.inputString1.Value = "TextBox";
			// 
			// inputSelection1
			// 
			this.inputSelection1.BackColor = System.Drawing.SystemColors.ControlDarkDark;
			this.inputSelection1.Font = new System.Drawing.Font("MS UI Gothic", 9F, System.Drawing.FontStyle.Regular, System.Drawing.GraphicsUnit.Point, ((byte)(128)));
			this.inputSelection1.Location = new System.Drawing.Point(79, 201);
			this.inputSelection1.Margin = new System.Windows.Forms.Padding(6, 5, 6, 5);
			this.inputSelection1.Name = "inputSelection1";
			this.inputSelection1.Size = new System.Drawing.Size(191, 22);
			this.inputSelection1.TabIndex = 3;
			this.inputSelection1.Value = "ComboBox";
			// 
			// inputHexadecimal1
			// 
			this.inputHexadecimal1.BackColor = System.Drawing.SystemColors.ControlDarkDark;
			this.inputHexadecimal1.Location = new System.Drawing.Point(79, 141);
			this.inputHexadecimal1.MaxLength = 28;
			this.inputHexadecimal1.Name = "inputHexadecimal1";
			this.inputHexadecimal1.Size = new System.Drawing.Size(256, 12);
			this.inputHexadecimal1.TabIndex = 2;
			this.inputHexadecimal1.Value = "0x";
			// 
			// inputFile1
			// 
			this.inputFile1.BackColor = System.Drawing.SystemColors.ControlDarkDark;
			this.inputFile1.Location = new System.Drawing.Point(79, 108);
			this.inputFile1.MaxDisplayLength = 10;
			this.inputFile1.MaxLength = 10;
			this.inputFile1.Name = "inputFile1";
			this.inputFile1.Size = new System.Drawing.Size(132, 12);
			this.inputFile1.TabIndex = 1;
			this.inputFile1.Value = "TextBox";
			// 
			// inputDecimal1
			// 
			this.inputDecimal1.BackColor = System.Drawing.SystemColors.ControlDarkDark;
			this.inputDecimal1.Location = new System.Drawing.Point(79, 67);
			this.inputDecimal1.Maximum = new decimal(new int[] {
            100,
            0,
            0,
            0});
			this.inputDecimal1.Minimum = new decimal(new int[] {
            100,
            0,
            0,
            -2147483648});
			this.inputDecimal1.Name = "inputDecimal1";
			this.inputDecimal1.Size = new System.Drawing.Size(76, 12);
			this.inputDecimal1.TabIndex = 0;
			this.inputDecimal1.Value = "0";
			// 
			// Form1
			// 
			this.AutoScaleDimensions = new System.Drawing.SizeF(6F, 12F);
			this.AutoScaleMode = System.Windows.Forms.AutoScaleMode.Font;
			this.ClientSize = new System.Drawing.Size(800, 450);
			this.Controls.Add(this.inputString1);
			this.Controls.Add(this.inputSelection1);
			this.Controls.Add(this.inputHexadecimal1);
			this.Controls.Add(this.inputFile1);
			this.Controls.Add(this.inputDecimal1);
			this.Name = "Form1";
			this.Text = "Form1";
			this.ResumeLayout(false);

		}

		#endregion

		private WinFormsCtrlLibInputScreen.InputDecimal inputDecimal1;
		private WinFormsCtrlLibInputScreen.InputFile inputFile1;
		private WinFormsCtrlLibInputScreen.InputHexadecimal inputHexadecimal1;
		private WinFormsCtrlLibInputScreen.InputSelection inputSelection1;
		private WinFormsCtrlLibInputScreen.InputString inputString1;
	}
}

