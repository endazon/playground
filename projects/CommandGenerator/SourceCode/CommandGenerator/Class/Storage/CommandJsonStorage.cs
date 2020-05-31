using Newtonsoft.Json;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;

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
			public bool Fixed { get; set; } = false;
			

			private List<string> convertFileData2CmdArray()
			{
				return convertFileData2CmdArray(Value);
			}
			private List<string> convertFileData2CmdArray(string value)
			{
				List<string> result = new List<string>();

				var buffer = new byte[Size];

				try
				{
					using (var reader = new BinaryReader(new FileStream(@value, FileMode.Open)))
					{
						reader.ReadByte(); //Todo
						reader.ReadByte(); //Todo
						reader.ReadByte(); //Todo
						reader.Read(buffer, 0, Size);
					}
				}
				catch (Exception e)
				{
					// ファイルを開くのに失敗したときエラーメッセージを表示
					Console.WriteLine(e.Message);
				}

				foreach (var item in buffer)
				{
					result.Add(item.ToString("X2"));
				}

				return result;
			}

			private List<string> convertAscii2CmdArray()
			{
				return convertAscii2CmdArray(Value);
			}
			private List<string> convertAscii2CmdArray(string value)
			{
				var list = new List<string>();

				foreach (var item in Encoding.ASCII.GetBytes(value))
				{
					list.Add(item.ToString("X2"));
				}

				return list;
			}

			private List<string> convertDec2CmdArray()
			{
				return convertDec2CmdArray(Value);
			}
			private List<string> convertDec2CmdArray(string value)
			{
				return convertHex2CmdArray(Convert.ToInt64(value).ToString("X" + (Size * 2).ToString()));
			}

			private List<string> convertHex2CmdArray()
			{
				return convertHex2CmdArray(Value);
			}
			private List<string> convertHex2CmdArray(string value)
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
			/// 詳細コピーメソッド
			/// </summary>
			/// <returns>コピーデータ</returns>
			public Parameter Clone()
			{
				// 簡易コピー
				Parameter cloned = (Parameter)MemberwiseClone();

				return cloned;
			}

			/// <summary>
			/// 電文正常判定
			/// </summary>
			/// <returns>結果</returns>
			public bool Verification(int baseSize)
			{
				if (Size < 0)                 { return false; }
				if (Offset + Size > baseSize) { return false; }
				return true;
			}

			/// <summary>
			/// 電文生成
			/// </summary>
			/// <returns>電文</returns>
			public List<string> Parser()
			{
				switch (Type)
				{
					case "FILE_SELECT":
						return convertFileData2CmdArray();
					case "ASCII":
						return convertAscii2CmdArray();
					case "DEC":
						return convertDec2CmdArray();
					case "HEX":
						return convertHex2CmdArray();
					default:
						throw new Exception();
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
			/// 電文正常判定
			/// </summary>
			/// <returns>結果</returns>
			public bool Verification(int baseSize)
			{
				if (Size < 0)                 { return false; }
				if (Offset + Size > baseSize) { return false; }

				foreach (var v in Parameter)
				{
					if (!v.Verification(Size)) { return false; }
				}
				return true;
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
					var list = p.Parser();
					buff.RemoveRange(p.Offset, list.Count);
					buff.InsertRange(p.Offset, list);
				}

				return buff;
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
			/// 電文正常判定
			/// </summary>
			/// <returns>結果</returns>
			public bool Verification()
			{
				if (Length < 0)                 { return false; }

				foreach (var v in Detail)
				{
					if (!v.Verification(Length)) { return false; }
				}
				return true;
			}

			/// <summary>
			/// 電文生成
			/// </summary>
			/// <returns>電文</returns>
			public List<string> Parser()
			{
				List<string> buff = new List<string>(Length);
				for (int index = 0; index < buff.Capacity; index++)
				{
					buff.Add("00");
				}

				foreach (var p in Detail)
				{
					var list = p.Parser();
					buff.RemoveRange(p.Offset, list.Count);
					buff.InsertRange(p.Offset, list);
				}

				return buff;
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

			/// <summary>
			/// シリアライズ
			/// </summary>
			/// <returns>t文字列</returns>
			public string Serialize()
			{
				return JsonConvert.SerializeObject(this);
			}

			/// <summary>
			/// デシリアライズ
			/// </summary>
			public void Deserialize(string target)
			{
				var obj = JsonConvert.DeserializeObject<CommandJsonObject>(target);
				Name    = obj.Name;
				Version = obj.Version;
				Items   = obj.Items;
			}

			/// <summary>
			/// 電文リスト生成
			/// </summary>
			/// <returns>電文</returns>
			public List<List<string>> Parser()
			{
				var list = new List<List<string>>(Items.Count);

				foreach (var p in Items)
				{
					list.Add(p.Parser());
				}

				return list;
			}

			/// <summary>
			/// 文字列化
			/// </summary>
			/// <returns>Name</returns>
			public override string ToString()
			{
				return Name;
			}

			/// <summary>
			/// 電文正常判定
			/// </summary>
			/// <returns>結果</returns>
			public bool Verification()
			{
				foreach (var v in Items)
				{
					if (!v.Verification()) { return false; }
				}
				return true;
			}
		}
	}
}
