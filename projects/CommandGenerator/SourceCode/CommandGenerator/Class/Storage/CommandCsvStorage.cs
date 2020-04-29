using CommandGenerator.Class.Storage;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace CommandCreator.Class.Storage
{
	class CommandCsvStorage
	{
		public class Detail
		{
			public string DisplayName { get; set; } = "";
			public string Name { get; set; } = "";
			public string Command { get; set; } = "";

			/// <summary>
			/// 詳細コピーメソッド
			/// </summary>
			/// <returns>コピーデータ</returns>
			public Detail Clone()
			{
				// 簡易コピー
				Detail cloned = (Detail)MemberwiseClone();

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
		}

		public class CommandCsvObject
		{
			public string Name { get; set; } = "";
			public string Version { get; set; } = "";

			public List<Detail> Items { get; set; } = new List<Detail>();

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
					List<Detail> ItemList = new List<Detail>();

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
		}
	}
}
