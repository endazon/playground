namespace DebuggingForms
{
    partial class DebuggingForm
    {
        /// <summary>
        ///  Required designer variable.
        /// </summary>
        private System.ComponentModel.IContainer components = null;

        /// <summary>
        ///  Clean up any resources being used.
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
        ///  Required method for Designer support - do not modify
        ///  the contents of this method with the code editor.
        /// </summary>
        private void InitializeComponent()
        {
            geometricDrawing1 = new Geometrical.GeometricDrawing();
            SuspendLayout();
            // 
            // geometricDrawing1
            // 
            geometricDrawing1.Dock = DockStyle.Fill;
            geometricDrawing1.Location = new Point(0, 0);
            geometricDrawing1.Name = "geometricDrawing1";
            geometricDrawing1.Size = new Size(800, 450);
            geometricDrawing1.TabIndex = 0;
            // 
            // DebuggingForm
            // 
            AutoScaleDimensions = new SizeF(7F, 15F);
            AutoScaleMode = AutoScaleMode.Font;
            ClientSize = new Size(800, 450);
            Controls.Add(geometricDrawing1);
            Name = "DebuggingForm";
            Text = "DebuggingForm";
            ResumeLayout(false);
        }

        #endregion

        private Geometrical.GeometricDrawing geometricDrawing1;
    }
}