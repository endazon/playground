namespace Geometrical
{
    partial class GeometricDrawing
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

        #region Component Designer generated code

        /// <summary>
        /// Required method for Designer support - do not modify 
        /// the contents of this method with the code editor.
        /// </summary>
        private void InitializeComponent()
        {
            Plane.CoordinateSystem coordinateSystem2 = new Plane.CoordinateSystem();
            System.ComponentModel.ComponentResourceManager resources = new System.ComponentModel.ComponentResourceManager(typeof(GeometricDrawing));
            splitContainer1 = new SplitContainer();
            statusStrip1 = new StatusStrip();
            Plane1 = new Plane.BasicPlane();
            ((System.ComponentModel.ISupportInitialize)splitContainer1).BeginInit();
            splitContainer1.Panel1.SuspendLayout();
            splitContainer1.Panel2.SuspendLayout();
            splitContainer1.SuspendLayout();
            ((System.ComponentModel.ISupportInitialize)Plane1).BeginInit();
            SuspendLayout();
            // 
            // splitContainer1
            // 
            splitContainer1.Dock = DockStyle.Fill;
            splitContainer1.IsSplitterFixed = true;
            splitContainer1.Location = new Point(0, 0);
            splitContainer1.Name = "splitContainer1";
            splitContainer1.Orientation = Orientation.Horizontal;
            // 
            // splitContainer1.Panel1
            // 
            splitContainer1.Panel1.Controls.Add(Plane1);
            // 
            // splitContainer1.Panel2
            // 
            splitContainer1.Panel2.Controls.Add(statusStrip1);
            splitContainer1.Size = new Size(800, 450);
            splitContainer1.SplitterDistance = 420;
            splitContainer1.TabIndex = 0;
            // 
            // statusStrip1
            // 
            statusStrip1.Location = new Point(0, 4);
            statusStrip1.Name = "statusStrip1";
            statusStrip1.Size = new Size(800, 22);
            statusStrip1.TabIndex = 0;
            statusStrip1.Text = "statusStrip1";
            // 
            // Plane1
            // 
            Plane1.Dock = DockStyle.Fill;
            Plane1.Location = new Point(0, 0);
            Plane1.Name = "Plane1";
            Plane1.Size = new Size(800, 420);
            coordinateSystem2.Direction = Plane.CoordinateDirections.RightHanded;
            coordinateSystem2.MagnificationRate = 1F;
            coordinateSystem2.Origin = (PointF)resources.GetObject("coordinateSystem2.Origin");
            coordinateSystem2.Rotation = Plane.CoordinateRotates.Angle000;
            coordinateSystem2.Scale = 1F;
            Plane1.System = coordinateSystem2;
            Plane1.TabIndex = 0;
            Plane1.TabStop = false;
            // 
            // GeometricDrawing
            // 
            AutoScaleDimensions = new SizeF(7F, 15F);
            AutoScaleMode = AutoScaleMode.Font;
            Controls.Add(splitContainer1);
            Name = "GeometricDrawing";
            Size = new Size(800, 450);
            splitContainer1.Panel1.ResumeLayout(false);
            splitContainer1.Panel2.ResumeLayout(false);
            splitContainer1.Panel2.PerformLayout();
            ((System.ComponentModel.ISupportInitialize)splitContainer1).EndInit();
            splitContainer1.ResumeLayout(false);
            ((System.ComponentModel.ISupportInitialize)Plane1).EndInit();
            ResumeLayout(false);
        }

        #endregion

        private SplitContainer splitContainer1;
        private StatusStrip statusStrip1;
        private Plane.BasicPlane Plane1;
    }
}