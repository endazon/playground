using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;
using CommandGenerator;
using CommandGenerator.Class.Storage;
using CommandGenerator.Target.Common.Range;
using CommandGenerator.Utility.InputDisplay.Facade;
using CommandGenerator.Utility.InputDisplay.Strategy;
using WinFormsCtrlLibInputScreen;
using WinFormsCtrlLibInputScreen.Base;

namespace ListSettings
{
	public partial class FormMain : Form
	{
		readonly Font FONT    = new Font("MS UI Gothic", 16F, FontStyle.Regular, GraphicsUnit.Point, 128);
		readonly int MAX_TEXT = 20;
		readonly int X        = 5;
		readonly int Y        = 15;

		public object CommandObj { get; set; } = new CommandJsonStorage.CommandJsonObject();

		public FormMain()
		{
			InitializeComponent();
		}
		public FormMain(string name, string ver) : this()
		{
			((CommandJsonStorage.CommandJsonObject)CommandObj).Name    = name;
			((CommandJsonStorage.CommandJsonObject)CommandObj).Version = ver;
			textBoxName.Text    = name;
			textBoxVersion.Text = ver;
		}

		public void Clear()
		{
			textBoxName.Text    = "";
			textBoxVersion.Text = "";
			treeView.Nodes.Clear();
		}
		public void AddRange(Control[] controls)
		{

		}
		public void Add(Control value)
		{

		}

		private void DisplayUpdate()
		{
			DisplayUpdate((CommandJsonStorage.CommandJsonObject)CommandObj);
		}

		private void DisplayUpdate(CommandJsonStorage.CommandJsonObject obj)
		{
			if (obj.GetType().Name != "CommandJsonObject") { return; }

			treeView.Nodes.Clear();

			//ヘッダー情報
			textBoxName.Text    = obj.Name;
			textBoxVersion.Text = obj.Version;

			//メイン情報
			var _list = new MyLayoutClass(new VerticalLayout(), splitContainer.Panel2);
			var _search = new Searcher();
			foreach (var item in obj.Controls)
			{
				{
					var _input = new List<UserControlInput>
					{
						_search.getInputDisplayOfType("ASCII").
						SetFont(FONT).
						SetName("Name").
						SetRange(new Range<int>(MAX_TEXT)).
						SetValue(item.Name).
						GetInputDisplay(),

						_search.getInputDisplayOfType("DEC").
						SetFont(FONT).
						SetName("Length").
						SetRange(new Range<int>(ushort.MaxValue)).
						SetValue(item.Length.ToString()).
						GetInputDisplay()
					};
					_list.Layout(_input);
				}

				TreeNode nodeItem = new TreeNode();
				nodeItem.NodeFont = FONT;
				nodeItem.Name = item.Name;
				nodeItem.Text = item.Name;
				nodeItem.Tag = _list.Clone();
				treeView.Nodes.Add(nodeItem);
				foreach (var detail in item.Controls)
				{
					{
						var _input = new List<UserControlInput>
						{
							_search.getInputDisplayOfType("ASCII").
							SetFont(FONT).
							SetName("Name").
							SetRange(new Range<int>(MAX_TEXT)).
							SetValue(detail.Name).
							GetInputDisplay(),

							_search.getInputDisplayOfType("DEC").
							SetFont(FONT).
							SetName("Offset").
							SetRange(new Range<int>(ushort.MaxValue)).
							SetValue(detail.Offset.ToString()).
							GetInputDisplay(),

							_search.getInputDisplayOfType("DEC").
							SetFont(FONT).
							SetName("Size").
							SetRange(new Range<int>(ushort.MaxValue)).
							SetValue(detail.Size.ToString()).
							GetInputDisplay()
						};
						_list.Layout(_input);
					}
					TreeNode nodeDetail = new TreeNode();
					nodeDetail.NodeFont = FONT;
					nodeDetail.Name = detail.Name;
					nodeDetail.Text = detail.Name;
					nodeDetail.Tag = _list.Clone();
					nodeItem.Nodes.Add(nodeDetail);
					foreach (var parameter in detail.Controls)
					{
						{
							var _input = new List<UserControlInput>
							{
								_search.getInputDisplayOfType("ASCII").
								SetFont(FONT).
								SetName("Name").
								SetRange(new Range<int>(MAX_TEXT)).
								SetValue(parameter.Name).
								GetInputDisplay(),

								_search.getInputDisplayOfType("DEC").
								SetFont(FONT).
								SetName("Offset").
								SetRange(new Range<int>(ushort.MaxValue)).
								SetValue(parameter.Offset.ToString()).
								GetInputDisplay(),

								_search.getInputDisplayOfType("DEC").
								SetFont(FONT).
								SetName("Size").
								SetRange(new Range<int>(ushort.MaxValue)).
								SetValue(parameter.Size.ToString()).
								GetInputDisplay(),

								_search.getInputDisplayOfType("SELECT").
								SetFont(FONT).
								SetName("Type").
								SetValue(parameter.Type).
								SetValues("HEX").
								SetValues("DEC").
								SetValues("ASCII").
								SetValues("FILE_SELECT").
								GetInputDisplay(),

								_search.getInputDisplayOfType("ASCII").
								SetFont(FONT).
								SetName("Value").
								SetRange(new Range<int>(MAX_TEXT)).
								SetValue(parameter.Value).
								GetInputDisplay(),

								_search.getInputDisplayOfType("SELECT").
								SetFont(FONT).
								SetName("Fixed").
								SetValue(parameter.Fixed == true ? "True" : "False").
								SetValues("True").
								SetValues("False").
								GetInputDisplay()
							};
							_list.Layout(_input);
						}

						TreeNode nodeParameter = new TreeNode();
						nodeParameter.NodeFont = FONT;
						nodeParameter.Name = parameter.Name;
						nodeParameter.Text = parameter.Name;
						nodeParameter.Tag = _list.Clone();
						nodeDetail.Nodes.Add(nodeParameter);
					}
				}
			}
			treeView.SelectedNode = treeView.TopNode;
		}

		private void Save()
		{
			var obj = new CommandJsonStorage.CommandJsonObject();

			obj.Name    = textBoxName.Text;
			obj.Version = textBoxVersion.Text;
			foreach (TreeNode item in treeView.Nodes)
			{
				var input_items = (MyLayoutClass)item.Tag;
				var items       = new CommandJsonStorage.Item();
				
				items.Name   = input_items.Search("Name").Text;
				items.Length = int.Parse(input_items.Search("Length").Text);
				foreach (TreeNode detail in item.Nodes)
				{
					var input_details = (MyLayoutClass)detail.Tag;
					var details       = new CommandJsonStorage.Detail();
					
					details.Name   = input_details.Search("Name").Text;
					details.Offset = int.Parse(input_details.Search("Offset").Text);
					details.Size   = int.Parse(input_details.Search("Size").Text);
					foreach (TreeNode parameter in detail.Nodes)
					{
						var input_parameter = (MyLayoutClass)parameter.Tag;
						var parameters      = new CommandJsonStorage.Parameter();
						
						parameters.Name   = input_parameter.Search("Name").Text;
						parameters.Offset = int.Parse(input_parameter.Search("Offset").Text);
						parameters.Size   = int.Parse(input_parameter.Search("Size").Text);
						parameters.Type   = input_parameter.Search("Type").Text;
						parameters.Value  = input_parameter.Search("Value").Text;
						parameters.Fixed  = input_parameter.Search("Fixed").Text == "True" ? true : false;
						details.Controls.Add(parameters);
					}
					items.Controls.Add(details);
				}
				obj.Controls.Add(items);
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
			var _list = new MyLayoutClass(new VerticalLayout(), splitContainer.Panel2);
			var _search = new Searcher();			
			var _input = new List<UserControlInput>
			{
				_search.getInputDisplayOfType("ASCII").
				SetFont(FONT).
				SetName("Name").
				SetRange(new Range<int>(MAX_TEXT)).
				SetValue(item.Name).
				GetInputDisplay(),

				_search.getInputDisplayOfType("DEC").
				SetFont(FONT).
				SetName("Length").
				SetRange(new Range<int>(ushort.MaxValue)).
				SetValue(item.Length.ToString()).
				GetInputDisplay()
			};
			_list.Layout(_input);

			TreeNode nodeItem = new TreeNode();
			nodeItem.NodeFont = FONT;
			nodeItem.Name = item.Name;
			nodeItem.Text = item.Name;
			nodeItem.Tag = _list;
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
			var _list = new MyLayoutClass(new VerticalLayout(), splitContainer.Panel2);
			var _search = new Searcher();
			var _input = new List<UserControlInput>
			{
				_search.getInputDisplayOfType("ASCII").
				SetFont(FONT).
				SetName("Name").
				SetRange(new Range<int>(MAX_TEXT)).
				SetValue(detail.Name).
				GetInputDisplay(),

				_search.getInputDisplayOfType("DEC").
				SetFont(FONT).
				SetName("Offset").
				SetRange(new Range<int>(ushort.MaxValue)).
				SetValue(detail.Offset.ToString()).
				GetInputDisplay(),

				_search.getInputDisplayOfType("DEC").
				SetFont(FONT).
				SetName("Size").
				SetRange(new Range<int>(ushort.MaxValue)).
				SetValue(detail.Size.ToString()).
				GetInputDisplay()
			};
			_list.Layout(_input);

			TreeNode nodeDetail = new TreeNode();
			nodeDetail.NodeFont = FONT;
			nodeDetail.Name = detail.Name;
			nodeDetail.Text = detail.Name;
			nodeDetail.Tag = _list;
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
			var _list = new MyLayoutClass(new VerticalLayout(), splitContainer.Panel2);
			var _search = new Searcher();
			var _input = new List<UserControlInput>
			{
				_search.getInputDisplayOfType("ASCII").
				SetFont(FONT).
				SetName("Name").
				SetRange(new Range<int>(MAX_TEXT)).
				SetValue(parameter.Name).
				GetInputDisplay(),

				_search.getInputDisplayOfType("DEC").
				SetFont(FONT).
				SetName("Offset").
				SetRange(new Range<int>(ushort.MaxValue)).
				SetValue(parameter.Offset.ToString()).
				GetInputDisplay(),

				_search.getInputDisplayOfType("DEC").
				SetFont(FONT).
				SetName("Size").
				SetRange(new Range<int>(ushort.MaxValue)).
				SetValue(parameter.Size.ToString()).
				GetInputDisplay(),

				_search.getInputDisplayOfType("SELECT").
				SetFont(FONT).
				SetName("Type").
				SetValue(parameter.Type).
				SetValues("HEX").
				SetValues("DEC").
				SetValues("ASCII").
				SetValues("FILE_SELECT").
				GetInputDisplay(),

				_search.getInputDisplayOfType("ASCII").
				SetFont(FONT).
				SetName("Value").
				SetRange(new Range<int>(MAX_TEXT)).
				SetValue(parameter.Value).
				GetInputDisplay(),

				_search.getInputDisplayOfType("SELECT").
				SetFont(FONT).
				SetName("Fixed").
				SetValue(parameter.Fixed == true ? "True" : "False").
				SetValues("True").
				SetValues("False").
				GetInputDisplay()
			};
			_list.Layout(_input);

			TreeNode nodeParameter = new TreeNode();
			nodeParameter.NodeFont = FONT;
			nodeParameter.Name = parameter.Name;
			nodeParameter.Text = parameter.Name;
			nodeParameter.Tag = _list;
			nodes.Insert(insertPos, nodeParameter);
		}


		#region TreeView
		private void treeView_AfterSelect(object sender, TreeViewEventArgs e)
		{
			var _list = (MyLayoutClass)e.Node.Tag;

			_list.Update();

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
