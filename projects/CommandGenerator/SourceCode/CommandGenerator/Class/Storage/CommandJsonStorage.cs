using System;
using System.Collections.Generic;

namespace CommandGenerator.Class.Storage
{
	class CommandJsonStorage
	{
		public class Parameter
		{
			public string Name { get; set; } = "";
			public int Offset { get; set; } = 0;
			public int Size { get; set; } = 0;
			public string Type { get; set; } = "";
			public string Value { get; set; } = "";
			public bool Hidden { get; set; } = false;


			/// <summary>
			/// 詳細コピーメソッド
			/// </summary>
			/// <returns>コピーデータ</returns>
			public Parameter Clone()
			{
				// 簡易コピー
				Parameter cloned = (Parameter)MemberwiseClone();

				return cloned;
			}

			private List<string> convertDec2HexArray(string type, List<string> value)
			{
				List<string> result = new List<string>(value.Count);
				for (int index = 0; index < value.Count; index++)
				{
					switch (type)
					{
						case "DEC":
							result.Add(Convert.ToInt32(value[index], 16).ToString("X2"));
							break;
						case "HEX":
						default:
							result.Add(value[index]);
							break;
					}
				}

				return result;
			}

			private List<string> convertValue2CmdArray(string value)
			{
				const int count = 2;
				string traget = value.Replace("0x", "");
				var list = new List<string>();
				int length = (int)Math.Ceiling((double)traget.Length / count);

				for (int i = 0; i < length; i++)
				{
					int start = count * i;
					if (traget.Length <= start)
					{
						break;
					}
					if (traget.Length < start + count)
					{
						list.Add(traget.Substring(start));
					}
					else
					{
						list.Add(traget.Substring(start, count));
					}
				}

				return list;
			}

			/// <summary>
			/// 電文生成
			/// </summary>
			/// <returns>電文</returns>
			public List<string> Parser()
			{
				return convertDec2HexArray(Type, convertValue2CmdArray(Value));
			}
		}

		public class Detail
		{
			public string Name { get; set; } = "";
			public int Offset { get; set; } = 0;
			public int Size { get; set; } = 0;
			public List<Parameter> Parameter { get; set; } = new List<Parameter>();


			/// <summary>
			/// 詳細コピーメソッド
			/// </summary>
			/// <returns>コピーデータ</returns>
			public Detail Clone()
			{
				// 簡易コピー
				Detail cloned = (Detail)MemberwiseClone();

				// 参照型フィールドの複製を作成する(簡易コピーを行う)
				if (Parameter != null)
				{
					List<Parameter> ParameterList = new List<Parameter>();

					foreach (var p in Parameter)
					{
						if (p != null)
						{
							ParameterList.Add(p.Clone());
						}
					}
					cloned.Parameter = ParameterList;
				}

				return cloned;
			}

			/// <summary>
			/// 電文生成
			/// </summary>
			/// <returns>電文</returns>
			public List<string> Parser()
			{
				List<string> buff = new List<string>(Size);
				for (int index = 0; index < buff.Capacity; index++)
				{
					buff.Add("00");
				}

				foreach (var p in Parameter)
				{
					buff.InsertRange(p.Offset, p.Parser());
				}

				return buff;
			}
		}

		public class Item
		{
			public string Name { get; set; } = "";
			public int Length { get; set; } = 0;
			public List<Detail> Detail { get; set; } = new List<Detail>();

			/// <summary>
			/// 詳細コピーメソッド
			/// </summary>
			/// <returns>コピーデータ</returns>
			public Item Clone()
			{
				// 簡易コピー
				Item cloned = (Item)MemberwiseClone();

				// 参照型フィールドの複製を作成する(簡易コピーを行う)
				if (Detail != null)
				{
					List<Detail> DetailList = new List<Detail>();

					foreach (var d in Detail)
					{
						if (d != null)
						{
							DetailList.Add(d.Clone());
						}
					}
					cloned.Detail = DetailList;
				}

				return cloned;
			}

			/// <summary>
			/// 電文生成
			/// </summary>
			/// <returns>電文</returns>
			public string Parser()
			{
				List<string> buff = new List<string>(Length);
				for (int index = 0; index < buff.Capacity; index++)
				{
					buff.Add("00");
				}

				foreach (var p in Detail)
				{
					buff.InsertRange(p.Offset, p.Parser());
				}

				return string.Join("", buff.ToArray());
			}
		}

		public class CommandJsonObject
		{
			public string Name { get; set; } = "";
			public string Version { get; set; } = "";
			public List<Item> Items { get; set; } = new List<Item>();

			/// <summary>
			/// 詳細コピーメソッド
			/// </summary>
			/// <returns>コピーデータ</returns>
			public CommandJsonObject Clone()
			{
				// 簡易コピー
				CommandJsonObject cloned = (CommandJsonObject)MemberwiseClone();

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
		}
	}
}
