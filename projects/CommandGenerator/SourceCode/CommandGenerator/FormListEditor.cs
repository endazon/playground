using System;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;
using CommandGenerator.Class.Storage;

namespace CommandGenerator
{
	public partial class FormListEditor : Form
	{
		readonly Font FONT    = new Font("MS UI Gothic", 16F, FontStyle.Regular, GraphicsUnit.Point, 128);
		readonly int MAX_TEXT = 20;
		readonly int X        = 5;
		readonly int Y        = 15;

		public object CommandObj { get; set; } = new CommandJsonStorage.CommandJsonObject();

		public FormListEditor()
		{
			InitializeComponent();
		}
		public FormListEditor(object obj)
			: this()
		{
			CommandObj = (CommandJsonStorage.CommandJsonObject)obj;
		}

		private void DisplayUpdate()
		{
			DisplayUpdate((CommandJsonStorage.CommandJsonObject)CommandObj);
		}

		private void DisplayUpdate(CommandJsonStorage.CommandJsonObject obj)
		{
			if (obj.GetType().Name != "CommandJsonObject") { return; }

			treeView.Nodes.Clear();

			textBoxName.Text    = obj.Name;
			textBoxVersion.Text = obj.Version;
			foreach (var item in obj.Items)
			{
				InputScreenStorage.InputGroup objItem = new InputScreenStorage.InputGroup();
				{
					InputScreenStorage.Input input = new InputScreenStorage.Input(
						"Name",
						FONT,
						new Point(X, Y),
						"ASCII",
						MAX_TEXT,
						item.Name
						);
					objItem.Input.Add(input);
				}
				{
					InputScreenStorage.Input input = new InputScreenStorage.Input(
						"Length",
						FONT,
						new Point(X, Y + (objItem.Input.Last().Label.Height + 10) * objItem.Input.Count),
						"DEC",
						4,
						item.Length.ToString()
						);
					objItem.Input.Add(input);
				}
				objItem.LineUp();

				TreeNode nodeItem = new TreeNode();
				nodeItem.NodeFont = FONT;
				nodeItem.Name = item.Name;
				nodeItem.Text = item.Name;
				nodeItem.Tag = objItem;
				treeView.Nodes.Add(nodeItem);
				foreach (var detail in item.Detail)
				{
					InputScreenStorage.InputGroup objDetail = new InputScreenStorage.InputGroup();
					{
						InputScreenStorage.Input input = new InputScreenStorage.Input(
						"Name",
						FONT,
						new Point(X, Y),
						"ASCII",
						MAX_TEXT,
						detail.Name
						);
						objDetail.Input.Add(input);
					}
					{
						InputScreenStorage.Input input = new InputScreenStorage.Input(
						"Offset",
						FONT,
						new Point(X, Y + (objDetail.Input.Last().Label.Height + 10) * objDetail.Input.Count),
						"DEC",
						4,
						detail.Offset.ToString()
						);
						objDetail.Input.Add(input);
					}
					{
						InputScreenStorage.Input input = new InputScreenStorage.Input(
						"Size",
						FONT,
						new Point(X, Y + (objDetail.Input.Last().Label.Height + 10) * objDetail.Input.Count),
						"DEC",
						4,
						detail.Size.ToString()
						);
						objDetail.Input.Add(input);
					}
					objDetail.LineUp();

					TreeNode nodeDetail = new TreeNode();
					nodeDetail.NodeFont = FONT;
					nodeDetail.Name = detail.Name;
					nodeDetail.Text = detail.Name;
					nodeDetail.Tag = objDetail;
					nodeItem.Nodes.Add(nodeDetail);
					foreach (var parameter in detail.Parameter)
					{
						InputScreenStorage.InputGroup objParameter = new InputScreenStorage.InputGroup();
						{
							InputScreenStorage.Input input = new InputScreenStorage.Input(
							"Name",
							FONT,
							new Point(X, Y),
							"ASCII",
							MAX_TEXT,
							parameter.Name
							);
							objParameter.Input.Add(input);
						}
						{
							InputScreenStorage.Input input = new InputScreenStorage.Input(
							"Offset",
							FONT,
							new Point(X, Y + (objParameter.Input.Last().Label.Height + 10) * objParameter.Input.Count),
							"DEC",
							4,
							parameter.Offset.ToString()
							);
							objParameter.Input.Add(input);
						}
						{
							InputScreenStorage.Input input = new InputScreenStorage.Input(
							"Size",
							FONT,
							new Point(X, Y + (objParameter.Input.Last().Label.Height + 10) * objParameter.Input.Count),
							"DEC",
							4,
							parameter.Size.ToString()
							);
							objParameter.Input.Add(input);
						}
						{
							InputScreenStorage.Input input = new InputScreenStorage.Input(
							"Type",
							FONT,
							new Point(X, Y + (objParameter.Input.Last().Label.Height + 10) * objParameter.Input.Count),
							"SELECT",
							4,
							parameter.Type
							);
							((ComboBox)input.InputBox).Items.Add("HEX");
							((ComboBox)input.InputBox).Items.Add("DEC");
							((ComboBox)input.InputBox).Items.Add("ASCII");
							((ComboBox)input.InputBox).Items.Add("FILE_SELECT");
							objParameter.Input.Add(input);
						}
						{
							InputScreenStorage.Input input = new InputScreenStorage.Input(
							"Value",
							FONT,
							new Point(X, Y + (objParameter.Input.Last().Label.Height + 10) * objParameter.Input.Count),
							"ASCII",
							4,
							parameter.Value
							);
							objParameter.Input.Add(input);
						}
						{
							InputScreenStorage.Input input = new InputScreenStorage.Input(
							"Fixed",
							FONT,
							new Point(X, Y + (objParameter.Input.Last().Label.Height + 10) * objParameter.Input.Count),
							"SELECT",
							MAX_TEXT,
							parameter.Fixed == true ? "True" : "False"
							);
							((ComboBox)input.InputBox).Items.Add("True");
							((ComboBox)input.InputBox).Items.Add("False");
							objParameter.Input.Add(input);
						}
						objParameter.LineUp();

						TreeNode nodeParameter = new TreeNode();
						nodeParameter.NodeFont = FONT;
						nodeParameter.Name = parameter.Name;
						nodeParameter.Text = parameter.Name;
						nodeParameter.Tag = objParameter;
						nodeDetail.Nodes.Add(nodeParameter);
					}
				}
			}
			treeView.SelectedNode = treeView.TopNode;
		}

		private void Save()
		{
			CommandJsonStorage.CommandJsonObject obj = new CommandJsonStorage.CommandJsonObject();

			obj.Name = textBoxName.Text;
			obj.Version = textBoxVersion.Text;
			foreach (TreeNode item in treeView.Nodes)
			{
				InputScreenStorage.InputGroup input_items = (InputScreenStorage.InputGroup)item.Tag;
				CommandJsonStorage.Item items = new CommandJsonStorage.Item();
				items.Name = input_items.Input[0].InputBox.Text;
				items.Length = int.Parse(input_items.Input[1].InputBox.Text);
				foreach (TreeNode detail in item.Nodes)
				{
					InputScreenStorage.InputGroup input_details = (InputScreenStorage.InputGroup)detail.Tag;
					CommandJsonStorage.Detail details = new CommandJsonStorage.Detail();
					details.Name = input_details.Input[0].InputBox.Text;
					details.Offset = int.Parse(input_details.Input[1].InputBox.Text);
					details.Size = int.Parse(input_details.Input[2].InputBox.Text);
					foreach (TreeNode parameter in detail.Nodes)
					{
						InputScreenStorage.InputGroup input_parameter = (InputScreenStorage.InputGroup)parameter.Tag;
						CommandJsonStorage.Parameter parameters = new CommandJsonStorage.Parameter();
						parameters.Name = input_parameter.Input[0].InputBox.Text;
						parameters.Offset = int.Parse(input_parameter.Input[1].InputBox.Text);
						parameters.Size = int.Parse(input_parameter.Input[2].InputBox.Text);
						parameters.Type = input_parameter.Input[3].InputBox.Text;
						parameters.Value = input_parameter.Input[4].InputBox.Text;
						parameters.Fixed = input_parameter.Input[5].InputBox.Text == "True" ? true : false;
						details.Parameter.Add(parameters);
					}
					items.Detail.Add(details);
				}
				obj.Items.Add(items);
			}

			CommandObj = obj.Clone();
		}

		private void AddCommand()
		{
			AddCommand(treeView.Nodes);
		}

		private void AddCommand(TreeNodeCollection nodes)
		{
			AddCommand(nodes, nodes.Count);
		}
		private void AddCommand(TreeNodeCollection nodes, int insertPos)
		{
			string commandName = new FormInputTextBox("コマンド名を入力して下さい。",
										  "コマンド名の入力",
										  "Command"
										  ).GetInputText();
			if (commandName != "")
			{
				try
				{
					if (nodes.Find(commandName, false).Length != 0) { throw new Exception(); }

					CommandJsonStorage.Item item = new CommandJsonStorage.Item();
					item.Name = commandName;
					item.Length = 1;

					AddCommand(nodes, insertPos, item);
				}
				catch
				{
					MessageBox.Show("名前が重複しています。\nコマンド作成を中断します。",
									"エラー",
									MessageBoxButtons.OK,
									MessageBoxIcon.Error);
				}
			}
		}
		private void AddCommand(TreeNodeCollection nodes, int insertPos, CommandJsonStorage.Item item)
		{
			InputScreenStorage.InputGroup objItem = new InputScreenStorage.InputGroup();
			{
				InputScreenStorage.Input input = new InputScreenStorage.Input(
					"Name",
					FONT,
					new Point(X, Y),
					"ASCII",
					MAX_TEXT,
					item.Name
					);
				objItem.Input.Add(input);
			}
			{
				InputScreenStorage.Input input = new InputScreenStorage.Input(
					"Length",
					FONT,
					new Point(X, Y + (objItem.Input.Last().Label.Height + 10) * objItem.Input.Count),
					"DEC",
					4,
					item.Length.ToString()
					);
				objItem.Input.Add(input);
			}
			objItem.LineUp();

			TreeNode nodeItem = new TreeNode();
			nodeItem.NodeFont = FONT;
			nodeItem.Name = item.Name;
			nodeItem.Text = item.Name;
			nodeItem.Tag = objItem;
			nodes.Insert(insertPos, nodeItem);

			AddDetail(nodeItem.Nodes);
		}

		private void AddDetail(TreeNodeCollection nodes)
		{
			AddDetail(nodes, nodes.Count);
		}
		private void AddDetail(TreeNodeCollection nodes, int insertPos)
		{
			CommandJsonStorage.Detail detail = new CommandJsonStorage.Detail();
			detail.Name = "DefaultDetail";
			detail.Offset = 0;
			detail.Size = 1;

			AddDetail(nodes, insertPos, detail);
		}
		private void AddDetail(TreeNodeCollection nodes, int insertPos, CommandJsonStorage.Detail detail)
		{
			InputScreenStorage.InputGroup objDetail = new InputScreenStorage.InputGroup();
			{
				InputScreenStorage.Input input = new InputScreenStorage.Input(
				"Name",
				FONT,
				new Point(X, Y),
				"ASCII",
				MAX_TEXT,
				detail.Name
				);
				objDetail.Input.Add(input);
			}
			{
				InputScreenStorage.Input input = new InputScreenStorage.Input(
				"Offset",
				FONT,
				new Point(X, Y + (objDetail.Input.Last().Label.Height + 10) * objDetail.Input.Count),
				"DEC",
				4,
				detail.Offset.ToString()
				);
				objDetail.Input.Add(input);
			}
			{
				InputScreenStorage.Input input = new InputScreenStorage.Input(
				"Size",
				FONT,
				new Point(X, Y + (objDetail.Input.Last().Label.Height + 10) * objDetail.Input.Count),
				"DEC",
				4,
				detail.Size.ToString()
				);
				objDetail.Input.Add(input);
			}
			objDetail.LineUp();

			TreeNode nodeDetail = new TreeNode();
			nodeDetail.NodeFont = FONT;
			nodeDetail.Name = detail.Name;
			nodeDetail.Text = detail.Name;
			nodeDetail.Tag = objDetail;
			nodes.Insert(insertPos, nodeDetail);

			AddParameter(nodeDetail.Nodes);
		}

		private void AddParameter(TreeNodeCollection nodes)
		{
			AddParameter(nodes, nodes.Count);
		}
		private void AddParameter(TreeNodeCollection nodes, int insertPos)
		{
			CommandJsonStorage.Parameter parameter = new CommandJsonStorage.Parameter();
			parameter.Name = "DefaultParameter";
			parameter.Offset = 0;
			parameter.Size = 1;
			parameter.Type = "HEX";
			parameter.Value = "0x00";
			parameter.Fixed = false;

			AddParameter(nodes, insertPos, parameter);
		}
		private void AddParameter(TreeNodeCollection nodes, int insertPos, CommandJsonStorage.Parameter parameter)
		{
			InputScreenStorage.InputGroup objParameter = new InputScreenStorage.InputGroup();
			{
				InputScreenStorage.Input input = new InputScreenStorage.Input(
				"Name",
				FONT,
				new Point(X, Y),
				"ASCII",
				MAX_TEXT,
				parameter.Name
				);
				objParameter.Input.Add(input);
			}
			{
				InputScreenStorage.Input input = new InputScreenStorage.Input(
				"Offset",
				FONT,
				new Point(X, Y + (objParameter.Input.Last().Label.Height + 10) * objParameter.Input.Count),
				"DEC",
				4,
				parameter.Offset.ToString()
				);
				objParameter.Input.Add(input);
			}
			{
				InputScreenStorage.Input input = new InputScreenStorage.Input(
				"Size",
				FONT,
				new Point(X, Y + (objParameter.Input.Last().Label.Height + 10) * objParameter.Input.Count),
				"DEC",
				4,
				parameter.Size.ToString()
				);
				objParameter.Input.Add(input);
			}
			{
				InputScreenStorage.Input input = new InputScreenStorage.Input(
				"Type",
				FONT,
				new Point(X, Y + (objParameter.Input.Last().Label.Height + 10) * objParameter.Input.Count),
				"SELECT",
				4,
				parameter.Type
				);
				((ComboBox)input.InputBox).Items.Add("HEX");
				((ComboBox)input.InputBox).Items.Add("DEC");
				((ComboBox)input.InputBox).Items.Add("ASCII");
				((ComboBox)input.InputBox).Items.Add("FILE_SELECT");
				objParameter.Input.Add(input);
			}
			{
				InputScreenStorage.Input input = new InputScreenStorage.Input(
				"Value",
				FONT,
				new Point(X, Y + (objParameter.Input.Last().Label.Height + 10) * objParameter.Input.Count),
				"ASCII",
				4,
				parameter.Value
				);
				objParameter.Input.Add(input);
			}
			{
				InputScreenStorage.Input input = new InputScreenStorage.Input(
				"Fixed",
				FONT,
				new Point(X, Y + (objParameter.Input.Last().Label.Height + 10) * objParameter.Input.Count),
				"SELECT",
				MAX_TEXT,
				parameter.Fixed == true ? "True" : "False"
				);
				((ComboBox)input.InputBox).Items.Add("True");
				((ComboBox)input.InputBox).Items.Add("False");

				objParameter.Input.Add(input);
			}
			objParameter.LineUp();

			TreeNode nodeParameter = new TreeNode();
			nodeParameter.NodeFont = FONT;
			nodeParameter.Name = parameter.Name;
			nodeParameter.Text = parameter.Name;
			nodeParameter.Tag = objParameter;
			nodes.Insert(insertPos, nodeParameter);
		}


		#region TreeView
		private void treeView_AfterSelect(object sender, TreeViewEventArgs e)
		{
			splitContainer.Panel2.Controls.Clear();
			foreach (var item in ((InputScreenStorage.InputGroup)e.Node.Tag).Input.ToArray())
			{
				splitContainer.Panel2.Controls.AddRange(item.ToArray());
			}

			//Console.WriteLine(e.Node.Text);
		}
		#endregion

		#region Button
		private void button_Click(object sender, EventArgs e)
		{
			AddCommand();
		}
		#endregion

		#region ContextMenuStrip
		private void addToolStripMenuItem_Click(object sender, EventArgs e)
		{
			int level       = treeView.SelectedNode.Level;
			int index       = treeView.SelectedNode.Index;
			TreeNode parent = treeView.SelectedNode.Parent;

			switch (level)
			{
				case 0:
					AddCommand(treeView.Nodes, index);
					break;
				case 1:
					AddDetail(parent.Nodes, index);
					break;
				case 2:
					AddParameter(parent.Nodes, index);
					break;
				default:
					throw new Exception();
			}
		}

		private void removeToolStripMenuItem_Click(object sender, EventArgs e)
		{
			TreeNode selected = treeView.SelectedNode;
			int level         = treeView.SelectedNode.Level;
			TreeNode parent   = treeView.SelectedNode.Parent;

			if ((level > 0)&&(parent.Nodes.Count <= 1))
			{
				MessageBox.Show("これ以上削除できません。",
								"エラー",
								MessageBoxButtons.OK,
								MessageBoxIcon.Error);
				return;
			}
			treeView.Nodes.Remove(selected);
		}
		#endregion

		#region Window
		private void FormListEditor_Shown(object sender, EventArgs e)
		{
			DisplayUpdate();
		}

		private void FormListEditor_FormClosed(object sender, FormClosedEventArgs e)
		{
			Save();
		}
		#endregion
	}
}
