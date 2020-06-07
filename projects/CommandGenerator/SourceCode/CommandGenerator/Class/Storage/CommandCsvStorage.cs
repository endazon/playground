using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace CommandGenerator.Class.Storage
{
	class CommandCsvStorage
	{
		public class Item
		{
			public string Name { get; set; } = "";
			public string Command { get; set; } = "";
			public ulong Length { get; set; } = 0;
			public string Type { get; set; } = "";
			public object Tag { get; set; } = null;

			/// <summary>
			/// 詳細コピーメソッド
			/// </summary>
			/// <returns>コピーデータ</returns>
			public Item Clone()
			{
				// 簡易コピー
				Item cloned = (Item)MemberwiseClone();

				return cloned;
			}

			/// <summary>
			/// 電文解析
			/// </summary>
			/// <para>Source of analysis</para>
			/// <returns>電文</returns>
			public CommandJsonStorage.Item Parser(CommandJsonStorage.Item src)
			{
				if(Name != src.Name) { }

				return null;
			}

			/// <summary>
			/// 文字列化
			/// </summary>
			/// <returns>Name</returns>
			public override string ToString()
			{
				return Name;
			}
		}

		public class CommandCsvObject
		{
			public string Name { get; set; } = "";
			public string Version { get; set; } = "";

			public List<Item> Items { get; set; } = new List<Item>();

			private List<string> SplitLineBreak(string target)
			{
				if (target.Contains('\n'))
				{
					return target.Replace("\r", "").Split('\n').ToList();
				}
				else if (target.Contains('\r'))
				{
					return target.Replace("\n", "").Split('\r').ToList();
				}
				else
				{
					throw new Exception();
				}
			}

			/// <summary>
			/// 詳細コピーメソッド
			/// </summary>
			/// <returns>コピーデータ</returns>
			public CommandCsvObject Clone()
			{
				// 簡易コピー
				CommandCsvObject cloned = (CommandCsvObject)MemberwiseClone();

				// 参照型フィールドの複製を作成する(簡易コピーを行う)
				if (Items != null)
				{
					List<Item> ItemList = new List<Item>();

					foreach (var i in Items)
					{
						if (i != null)
						{
							ItemList.Add(i.Clone());
						}
					}
					cloned.Items = ItemList;
				}

				return cloned;
			}

			/// <summary>
			/// シリアライズ
			/// </summary>
			/// <returns>t文字列</returns>
			public string Serialize()
			{
				var result = "";
				result +="Name,"     + Name    + "\n";
				result += "Version," + Version + "\n";
				result += ",\n";
				result += "No., Name, Command, Length, Type\n";
				foreach (var list in Items)
				{
					result += (Items.IndexOf(list) + 1).ToString() + ","
								+ list.Name		+ ","
								+ list.Command	+ ","
								+ list.Length	+ ","
								+ list.Type
								+ "\n";
				}
				return result;
			}

			/// <summary>
			/// デシリアライズ
			/// </summary>
			public void Deserialize(string target)
			{
				var list = SplitLineBreak(target);

				Name    = list.First().Split(',')[1]; list.RemoveAt(0);
				Version = list.First().Split(',')[1]; list.RemoveAt(0);
				list.RemoveAt(0);
				list.RemoveAt(0);

				foreach (var list_item in list)
				{
					if (list_item == "") { continue; }
					var item     = new Item();
					item.Name    = list_item.Split(',')[1];
					item.Command = list_item.Split(',')[2];
					item.Length  = Convert.ToUInt64(list_item.Split(',')[3]);
					item.Type    = list_item.Split(',')[4];

					Items.Add(item);
				}
			}

			/// <summary>
			/// 文字列化
			/// </summary>
			/// <returns>Name</returns>
			public override string ToString()
			{
				return Name;
			}
		}
	}
}
