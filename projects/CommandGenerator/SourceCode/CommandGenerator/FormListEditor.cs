using CommandGenerator.Class.Storage;
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Data;
using System.Drawing;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace CommandGenerator
{
	public partial class FormListEditor : Form
	{
		private new FormList Owner { get; set; }

		public FormListEditor()
		{
			InitializeComponent();
		}
		public FormListEditor(FormList owner)
			:this()
		{
			Owner = owner;
		}

		private Size GetOneCharacterSize(Font font)
		{
			Label label = new Label();
			label.AutoSize = true;
			label.Font = font;
			label.Name = " ";
			label.Text = " ";
			return new Size(label.PreferredWidth - 5, label.PreferredHeight);
		}

		public void update()
		{
			Font FONT = new Font("MS UI Gothic", 16F, FontStyle.Regular, GraphicsUnit.Point, 128);
			int X     = 5;
			int Y     = 15;

			treeView.Nodes.Clear();

			textBoxName.Text    = Owner.CommandObj.Name;
			textBoxVersion.Text = Owner.CommandObj.Version;
			foreach (var item in Owner.CommandObj.Items)
			{
				List<InputScreenStorage.Input> objTop = new List<InputScreenStorage.Input>();

				{
					InputScreenStorage.Input input = new InputScreenStorage.Input();

					Label label       = new Label();
					label.AutoSize    = true;
					label.BorderStyle = BorderStyle.FixedSingle;
					label.Font        = FONT;
					label.Name        = "Name";
					label.Text        = "Name";
					label.Location    = new Point(X, Y);
					input.Label       = label;

					TextBox textbox     = new TextBox();
					textbox.BackColor   = SystemColors.WindowText;
					textbox.BorderStyle = BorderStyle.FixedSingle;
					textbox.Font        = FONT;
					textbox.ForeColor   = SystemColors.GrayText;
					textbox.Name        = "NameBox";
					textbox.Text        = item.Name;
					textbox.Location    = new Point(X + label.PreferredWidth, Y);
					textbox.Size        = new Size(GetOneCharacterSize(FONT).Width*20, label.PreferredHeight);
					input.InputBox      = textbox;

					objTop.Add(input);
				}
				{
					InputScreenStorage.Input input = new InputScreenStorage.Input();

					Label label       = new Label();
					label.AutoSize    = true;
					label.BorderStyle = BorderStyle.FixedSingle;
					label.Font        = FONT;
					label.Name        = "Length";
					label.Text        = "Length";
					label.Location    = new Point(X, Y+ GetOneCharacterSize(FONT).Height+10);
					input.Label       = label;

					NumericUpDown numericupdown = new NumericUpDown();
					numericupdown.BackColor     = SystemColors.WindowText;
					numericupdown.BorderStyle   = BorderStyle.FixedSingle;
					numericupdown.Font          = FONT;
					numericupdown.ForeColor     = SystemColors.GrayText;
					numericupdown.Minimum       = 0;
					numericupdown.Maximum       = (int)Math.Pow(2, (8 * 4)) - 1;//4Byte
					numericupdown.Name          = "LengthBox";
					numericupdown.Text          = item.Length.ToString();
					numericupdown.Location      = new Point(X + label.PreferredWidth, Y + GetOneCharacterSize(FONT).Height + 10);
					int digits                  = (int)Math.Log10((double)numericupdown.Maximum) + 1;
					numericupdown.Size          = new Size(GetOneCharacterSize(FONT).Width * (digits + 1), label.PreferredHeight);
					input.InputBox              = numericupdown;

					objTop.Add(input);
				}

				TreeNode nodeTop    = new TreeNode();
				nodeTop.NodeFont    = FONT;
				nodeTop.Name        = item.Name;
				nodeTop.Text        = item.Name;
				nodeTop.Tag         = objTop;
				treeView.Nodes.Add(nodeTop);
				foreach (var detail in item.Detail)
				{
					List<InputScreenStorage.Input> objDetail = new List<InputScreenStorage.Input>();

					{
						InputScreenStorage.Input input = new InputScreenStorage.Input();

						Label label       = new Label();
						label.AutoSize    = true;
						label.BorderStyle = BorderStyle.FixedSingle;
						label.Font        = FONT;
						label.Name        = "Name";
						label.Text        = "Name";
						label.Location    = new Point(X, Y);
						input.Label       = label;

						TextBox textbox     = new TextBox();
						textbox.BackColor   = SystemColors.WindowText;
						textbox.BorderStyle = BorderStyle.FixedSingle;
						textbox.Font        = FONT;
						textbox.ForeColor   = SystemColors.GrayText;
						textbox.Name        = "NameBox";
						textbox.Text        = detail.Name;
						textbox.Location    = new Point(X + label.PreferredWidth, Y);
						textbox.Size        = new Size(GetOneCharacterSize(FONT).Width * 20, label.PreferredHeight);
						input.InputBox      = textbox;

						objDetail.Add(input);
					}
					{
						InputScreenStorage.Input input = new InputScreenStorage.Input();

						Label label       = new Label();
						label.AutoSize    = true;
						label.BorderStyle = BorderStyle.FixedSingle;
						label.Font        = FONT;
						label.Name        = "Offset";
						label.Text        = "Offset";
						label.Location    = new Point(X, Y + GetOneCharacterSize(FONT).Height + 10);
						input.Label       = label;

						NumericUpDown numericupdown = new NumericUpDown();
						numericupdown.BackColor     = SystemColors.WindowText;
						numericupdown.BorderStyle   = BorderStyle.FixedSingle;
						numericupdown.Font          = FONT;
						numericupdown.ForeColor     = SystemColors.GrayText;
						numericupdown.Minimum       = 0;
						numericupdown.Maximum       = (int)Math.Pow(2, (8 * 4)) - 1;//4Byte
						numericupdown.Name          = "OffsetBox";
						numericupdown.Text          = detail.Offset.ToString();
						numericupdown.Location      = new Point(X + label.PreferredWidth, Y + GetOneCharacterSize(FONT).Height + 10);
						int digits                  = (int)Math.Log10((double)numericupdown.Maximum) + 1;
						numericupdown.Size          = new Size(GetOneCharacterSize(FONT).Width * (digits + 1), label.PreferredHeight);
						input.InputBox              = numericupdown;

						objDetail.Add(input);
					}
					{
						InputScreenStorage.Input input = new InputScreenStorage.Input();

						Label label       = new Label();
						label.AutoSize    = true;
						label.BorderStyle = BorderStyle.FixedSingle;
						label.Font        = FONT;
						label.Name        = "Size";
						label.Text        = "Size";
						label.Location    = new Point(X, Y + (GetOneCharacterSize(FONT).Height + 10)*2);
						input.Label       = label;

						NumericUpDown numericupdown = new NumericUpDown();
						numericupdown.BackColor     = SystemColors.WindowText;
						numericupdown.BorderStyle   = BorderStyle.FixedSingle;
						numericupdown.Font          = FONT;
						numericupdown.ForeColor     = SystemColors.GrayText;
						numericupdown.Minimum       = 0;
						numericupdown.Maximum       = (int)Math.Pow(2, (8 * 4)) - 1;//4Byte
						numericupdown.Name          = "SizeBox";
						numericupdown.Text          = detail.Size.ToString();
						numericupdown.Location      = new Point(X + label.PreferredWidth, Y + (GetOneCharacterSize(FONT).Height + 10) * 2);
						int digits                  = (int)Math.Log10((double)numericupdown.Maximum) + 1;
						numericupdown.Size          = new Size(GetOneCharacterSize(FONT).Width * (digits + 1), label.PreferredHeight);
						input.InputBox              = numericupdown;

						objDetail.Add(input);
					}

					TreeNode nodeDetail = new TreeNode();
					nodeDetail.NodeFont = FONT;
					nodeDetail.Name     = detail.Name;
					nodeDetail.Text     = detail.Name;
					nodeDetail.Tag      = objDetail;
					nodeTop.Nodes.Add(nodeDetail);
					foreach (var parameter in detail.Parameter)
					{
						List<InputScreenStorage.Input> objParameter = new List<InputScreenStorage.Input>();

						{
							InputScreenStorage.Input input = new InputScreenStorage.Input();

							Label label       = new Label();
							label.AutoSize    = true;
							label.BorderStyle = BorderStyle.FixedSingle;
							label.Font        = FONT;
							label.Name        = "Name";
							label.Text        = "Name";
							label.Location    = new Point(X, Y);
							input.Label       = label;

							TextBox textbox     = new TextBox();
							textbox.BackColor   = SystemColors.WindowText;
							textbox.BorderStyle = BorderStyle.FixedSingle;
							textbox.Font        = FONT;
							textbox.ForeColor   = SystemColors.GrayText;
							textbox.Name        = "NameBox";
							textbox.Text        = parameter.Name;
							textbox.Location    = new Point(X + label.PreferredWidth, Y);
							textbox.Size        = new Size(GetOneCharacterSize(FONT).Width * 20, label.PreferredHeight);
							input.InputBox      = textbox;

							objParameter.Add(input);
						}
						{
							InputScreenStorage.Input input = new InputScreenStorage.Input();

							Label label       = new Label();
							label.AutoSize    = true;
							label.BorderStyle = BorderStyle.FixedSingle;
							label.Font        = FONT;
							label.Name        = "Offset";
							label.Text        = "Offset";
							label.Location    = new Point(X, Y + GetOneCharacterSize(FONT).Height + 10);
							input.Label       = label;

							NumericUpDown numericupdown = new NumericUpDown();
							numericupdown.BackColor     = SystemColors.WindowText;
							numericupdown.BorderStyle   = BorderStyle.FixedSingle;
							numericupdown.Font          = FONT;
							numericupdown.ForeColor     = SystemColors.GrayText;
							numericupdown.Minimum       = 0;
							numericupdown.Maximum       = (int)Math.Pow(2, (8 * 4)) - 1;//4Byte
							numericupdown.Name          = "OffsetBox";
							numericupdown.Text          = parameter.Offset.ToString();
							numericupdown.Location      = new Point(X + label.PreferredWidth, Y + GetOneCharacterSize(FONT).Height + 10);
							int digits                  = (int)Math.Log10((double)numericupdown.Maximum) + 1;
							numericupdown.Size          = new Size(GetOneCharacterSize(FONT).Width * (digits + 1), label.PreferredHeight);
							input.InputBox              = numericupdown;

							objParameter.Add(input);
						}
						{
							InputScreenStorage.Input input = new InputScreenStorage.Input();

							Label label       = new Label();
							label.AutoSize    = true;
							label.BorderStyle = BorderStyle.FixedSingle;
							label.Font        = FONT;
							label.Name        = "Size";
							label.Text        = "Size";
							label.Location    = new Point(X, Y + (GetOneCharacterSize(FONT).Height + 10) * 2);
							input.Label       = label;

							NumericUpDown numericupdown = new NumericUpDown();
							numericupdown.BackColor     = SystemColors.WindowText;
							numericupdown.BorderStyle   = BorderStyle.FixedSingle;
							numericupdown.Font          = FONT;
							numericupdown.ForeColor     = SystemColors.GrayText;
							numericupdown.Minimum       = 0;
							numericupdown.Maximum       = (int)Math.Pow(2, (8 * 4)) - 1;//4Byte
							numericupdown.Name          = "SizeBox";
							numericupdown.Text          = parameter.Size.ToString();
							numericupdown.Location      = new Point(X + label.PreferredWidth, Y + (GetOneCharacterSize(FONT).Height + 10) * 2);
							int digits                  = (int)Math.Log10((double)numericupdown.Maximum) + 1;
							numericupdown.Size          = new Size(GetOneCharacterSize(FONT).Width * (digits + 1), label.PreferredHeight);
							input.InputBox              = numericupdown;

							objParameter.Add(input);
						}
						{
							InputScreenStorage.Input input = new InputScreenStorage.Input();

							Label label       = new Label();
							label.AutoSize    = true;
							label.BorderStyle = BorderStyle.FixedSingle;
							label.Font        = FONT;
							label.Name        = "Type";
							label.Text        = "Type";
							label.Location    = new Point(X, Y + (GetOneCharacterSize(FONT).Height + 10) * 3);
							input.Label       = label;


							TextBox textbox     = new TextBox();
							textbox.BackColor   = SystemColors.WindowText;
							textbox.BorderStyle = BorderStyle.FixedSingle;
							textbox.Font        = FONT;
							textbox.ForeColor   = SystemColors.GrayText;
							textbox.Name        = "TypeBox";
							textbox.Text        = parameter.Type;
							textbox.Location    = new Point(X + label.PreferredWidth, Y + (GetOneCharacterSize(FONT).Height + 10) * 3);
							textbox.Size        = new Size(GetOneCharacterSize(FONT).Width * 20, label.PreferredHeight);
							input.InputBox      = textbox;

							objParameter.Add(input);
						}
						{
							InputScreenStorage.Input input = new InputScreenStorage.Input();

							Label label       = new Label();
							label.AutoSize    = true;
							label.BorderStyle = BorderStyle.FixedSingle;
							label.Font        = FONT;
							label.Name        = "Value";
							label.Text        = "Value";
							label.Location    = new Point(X, Y + (GetOneCharacterSize(FONT).Height + 10) * 4);
							input.Label       = label;

							TextBox textbox     = new TextBox();
							textbox.BackColor   = SystemColors.WindowText;
							textbox.BorderStyle = BorderStyle.FixedSingle;
							textbox.Font        = FONT;
							textbox.ForeColor   = SystemColors.GrayText;
							textbox.Name        = "ValueBox";
							textbox.Text        = parameter.Value;
							textbox.Location    = new Point(X + label.PreferredWidth, Y + (GetOneCharacterSize(FONT).Height + 10) * 4);
							textbox.Size        = new Size(GetOneCharacterSize(FONT).Width * 20, label.PreferredHeight);
							input.InputBox      = textbox;

							objParameter.Add(input);
						}
						{
							InputScreenStorage.Input input = new InputScreenStorage.Input();

							Label label       = new Label();
							label.AutoSize    = true;
							label.BorderStyle = BorderStyle.FixedSingle;
							label.Font        = FONT;
							label.Name        = "Fixed";
							label.Text        = "Fixed";
							label.Location    = new Point(X, Y + (GetOneCharacterSize(FONT).Height + 10) * 5);
							input.Label       = label;

							ComboBox combobox    = new ComboBox();
							combobox.BackColor   = SystemColors.WindowText;
							combobox.FlatStyle   = FlatStyle.Flat;
							combobox.Font        = FONT;
							combobox.ForeColor   = SystemColors.GrayText;
							combobox.Items.Add("True");
							combobox.Items.Add("False");
							combobox.Name        = "FixedBox";
							combobox.Text        = parameter.Fixed == true ? "True" : "False";
							combobox.Location    = new Point(X + label.PreferredWidth, Y + (GetOneCharacterSize(FONT).Height + 10) * 5);
							combobox.Size        = new Size(GetOneCharacterSize(FONT).Width * 20, label.PreferredHeight);
							input.InputBox       = combobox;

							objParameter.Add(input);
						}

						TreeNode nodeParameter = new TreeNode();
						nodeParameter.NodeFont = FONT;
						nodeParameter.Name     = parameter.Name;
						nodeParameter.Text     = parameter.Name;
						nodeParameter.Tag      = objParameter;
						nodeDetail.Nodes.Add(nodeParameter);
					}
				}
			}
			treeView.SelectedNode = treeView.TopNode;
		}

		public void save()
		{
			CommandJsonStorage.CommandJsonObject obj = new CommandJsonStorage.CommandJsonObject();

			obj.Name = textBoxName.Text;
			obj.Version = textBoxVersion.Text;
			foreach (TreeNode item in treeView.Nodes)
			{
				List<InputScreenStorage.Input> input_items = (List<InputScreenStorage.Input>)item.Tag;
				CommandJsonStorage.Item items              = new CommandJsonStorage.Item();
				items.Name                                 = input_items[0].InputBox.Text;
				items.Length                               = int.Parse(input_items[1].InputBox.Text);
				foreach (TreeNode detail in item.Nodes)
				{
					List<InputScreenStorage.Input> input_details = (List<InputScreenStorage.Input>)detail.Tag;
					CommandJsonStorage.Detail details            = new CommandJsonStorage.Detail();
					details.Name                                 = input_details[0].InputBox.Text;
					details.Offset                               = int.Parse(input_details[1].InputBox.Text);
					details.Size                                 = int.Parse(input_details[2].InputBox.Text);
					foreach (TreeNode parameter in detail.Nodes)
					{
						List<InputScreenStorage.Input> input_parameter    = (List<InputScreenStorage.Input>)parameter.Tag;
						CommandJsonStorage.Parameter parameters           = new CommandJsonStorage.Parameter();
						parameters.Name                                   = input_parameter[0].InputBox.Text;
						parameters.Offset                                 = int.Parse(input_parameter[1].InputBox.Text);
						parameters.Size                                   = int.Parse(input_parameter[2].InputBox.Text);
						parameters.Type                                   = input_parameter[3].InputBox.Text;
						parameters.Value                                  = input_parameter[4].InputBox.Text;
						parameters.Fixed                                  = input_parameter[5].InputBox.Text == "True"?true:false;
						details.Parameter.Add(parameters);
					}
					items.Detail.Add(details);
				}
				obj.Items.Add(items);
			}

			Owner.CommandObj = obj.Clone();
		}

		#region TreeView
		private void treeView_AfterSelect(object sender, TreeViewEventArgs e)
		{
			splitContainer.Panel2.Controls.Clear();
			foreach (var item in ((List<InputScreenStorage.Input>)e.Node.Tag).ToArray())
			{
				splitContainer.Panel2.Controls.AddRange(item.ToArray());
			}

			Console.WriteLine(e.Node.Text);
		}
		#endregion

		#region ContextMenuStrip
		private void addToolStripMenuItem_Click(object sender, EventArgs e)
		{

		}

		private void removeToolStripMenuItem_Click(object sender, EventArgs e)
		{
			if(treeView.SelectedNode != null)
			{
				treeView.Nodes.Remove(treeView.SelectedNode);
			}
			else
			{
				treeView.Nodes.Clear();
			}
		}
		#endregion

		#region Window
		private void FormListEditor_Shown(object sender, EventArgs e)
		{
			update();
		}

		private void FormListEditor_FormClosed(object sender, FormClosedEventArgs e)
		{
			save();
			Owner.FormListEditor_FormClosed(sender, e);
		}
		#endregion
	}
}
