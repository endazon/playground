using Newtonsoft.Json;
using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

using Extension;

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
					using (var reader = new BinaryReader(new FileStream(@value, FileMode.Open), Encoding.UTF8))
					{
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
				return value.NumStringSplit();
			}

			private string convertCmdArray2String(List<string> cmdArray)
			{
				return string.Join("", cmdArray.ToArray());
			}
			private string convertCmdArray2FileData(List<string> cmdArray)
			{
				var write_path = Environment.CurrentDirectory;
				var buffer = new List<byte>();

				var count = 0;
				foreach(var file in Directory.GetFiles(write_path))
				{
					count += Path.GetFileName(file).Contains(Name) == true ? 1 : 0;
				}
				write_path += "\\FileData_" + Name + count.ToString() + ".bin";

				foreach (var item in cmdArray)
				{
					buffer.Add(Convert.ToByte(item, 16));
				}

				try
				{
					using (var reader = new BinaryWriter(new FileStream(@write_path, FileMode.Create), Encoding.UTF8))
					{
						reader.Write(buffer.ToArray(), 0, Size);
					}
				}
				catch (Exception e)
				{
					// ファイルを開くのに失敗したときエラーメッセージを表示
					Console.WriteLine(e.Message);
				}

				return write_path;
			}
			private string convertCmdArray2Ascii(List<string> cmdArray)
			{
				var list = new List<byte>();

				foreach (var item in cmdArray)
				{
					list.Add(Convert.ToByte(item, 16));
				}
				list.RemoveAll(b => b == 0x00);

				return Encoding.ASCII.GetString(list.ToArray());
			}
			private string convertCmdArray2Dec(List<string> cmdArray)
			{
				return Convert.ToInt64(convertCmdArray2String(cmdArray), 16).ToString();
			}
			private string convertCmdArray2Hex(List<string> cmdArray)
			{
				return "0x" + convertCmdArray2String(cmdArray);
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
			public List<string> MessageGeneration()
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
			/// 電文解析
			/// </summary>
			/// <returns>電文</returns>
			public void Parser(List<string> target)
			{
				var data = target.GetRange(Offset, Size);

				switch (Type)
				{
					case "FILE_SELECT":
						Value = convertCmdArray2FileData(data);
						break;
					case "ASCII":
						Value = convertCmdArray2Ascii(data);
						break;
					case "DEC":
						Value = convertCmdArray2Dec(data);
						break;
					case "HEX":
						Value = convertCmdArray2Hex(data);
						break;
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
			[JsonProperty("Parameter")]
			public List<Parameter> Controls { get; set; } = new List<Parameter>();

			/// <summary>
			/// 詳細コピーメソッド
			/// </summary>
			/// <returns>コピーデータ</returns>
			public Detail Clone()
			{
				// 簡易コピー
				Detail cloned = (Detail)MemberwiseClone();

				// 参照型フィールドの複製を作成する(簡易コピーを行う)
				if (Controls != null)
				{
					List<Parameter> ParameterList = new List<Parameter>();

					foreach (var p in Controls)
					{
						if (p != null)
						{
							ParameterList.Add(p.Clone());
						}
					}
					cloned.Controls = ParameterList;
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

				foreach (var v in Controls)
				{
					if (!v.Verification(Size)) { return false; }
				}
				return true;
			}

			/// <summary>
			/// 電文生成
			/// </summary>
			/// <returns>電文</returns>
			public List<string> MessageGeneration()
			{
				List<string> buff = new List<string>(Size);
				for (int index = 0; index < buff.Capacity; index++)
				{
					buff.Add("00");
				}

				foreach (var p in Controls)
				{
					var list = p.MessageGeneration();
					buff.RemoveRange(p.Offset, list.Count);
					buff.InsertRange(p.Offset, list);
				}

				return buff;
			}

			/// <summary>
			/// 電文解析
			/// </summary>
			/// <returns>電文</returns>
			public void Parser(List<string> target)
			{
				var data = target.GetRange(Offset, Size);

				foreach (var p in Controls)
				{
					p.Parser(data);
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

		public class Item
		{
			public string Name { get; set; } = "";
			public int Length { get; set; } = 0;
			[JsonProperty("Detail")]
			public List<Detail> Controls { get; set; } = new List<Detail>();

			/// <summary>
			/// 詳細コピーメソッド
			/// </summary>
			/// <returns>コピーデータ</returns>
			public Item Clone()
			{
				// 簡易コピー
				Item cloned = (Item)MemberwiseClone();

				// 参照型フィールドの複製を作成する(簡易コピーを行う)
				if (Controls != null)
				{
					List<Detail> DetailList = new List<Detail>();

					foreach (var d in Controls)
					{
						if (d != null)
						{
							DetailList.Add(d.Clone());
						}
					}
					cloned.Controls = DetailList;
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

				foreach (var v in Controls)
				{
					if (!v.Verification(Length)) { return false; }
				}
				return true;
			}

			/// <summary>
			/// 電文生成
			/// </summary>
			/// <returns>電文</returns>
			public List<string> MessageGeneration()
			{
				List<string> buff = new List<string>(Length);
				for (int index = 0; index < buff.Capacity; index++)
				{
					buff.Add("00");
				}

				foreach (var p in Controls)
				{
					var list = p.MessageGeneration();
					buff.RemoveRange(p.Offset, list.Count);
					buff.InsertRange(p.Offset, list);
				}

				return buff;
			}

			/// <summary>
			/// 電文解析
			/// </summary>
			/// <returns>電文</returns>
			public void Parser(List<string> target)
			{
				if (target.Count != Length) { throw new Exception(); }

				foreach (var d in Controls)
				{
					d.Parser(target);
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

		public class CommandJsonObject
		{
			public string Name { get; set; } = "";
			public string Version { get; set; } = "";
			[JsonProperty("Items")]
			public List<Item> Controls { get; set; } = new List<Item>();

			/// <summary>
			/// 詳細コピーメソッド
			/// </summary>
			/// <returns>コピーデータ</returns>
			public CommandJsonObject Clone()
			{
				// 簡易コピー
				CommandJsonObject cloned = (CommandJsonObject)MemberwiseClone();

				// 参照型フィールドの複製を作成する(簡易コピーを行う)
				if (Controls != null)
				{
					List<Item> ItemList = new List<Item>();

					foreach (var i in Controls)
					{
						if (i != null)
						{
							ItemList.Add(i.Clone());
						}
					}
					cloned.Controls = ItemList;
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
				var obj  = JsonConvert.DeserializeObject<CommandJsonObject>(target);
				Name     = obj.Name;
				Version  = obj.Version;
				Controls = obj.Controls;
			}

			/// <summary>
			/// 電文リスト生成
			/// </summary>
			/// <returns>電文</returns>
			public List<List<string>> MessageGeneration()
			{
				var list = new List<List<string>>(Controls.Count);

				foreach (var p in Controls)
				{
					list.Add(p.MessageGeneration());
				}

				return list;
			}

			/// <summary>
			/// 電文解析
			/// </summary>
			/// <returns>電文</returns>
			public CommandCsvStorage.CommandCsvObject Parser(object target)
			{
				var obj = Clone();
				var data = (CommandCsvStorage.CommandCsvObject)target;
				var ret_data = data.Clone();
				if (data.Name    != Name)    { throw new Exception(); }
				if (data.Version != Version) { throw new Exception(); }

				ret_data.Items.Clear();
				foreach (var dst_item in data.Items)
				{
					foreach (var src_item in obj.Controls)
					{
						if (src_item.Name != dst_item.Type) { continue; }
						src_item.Parser(dst_item.Command.NumStringSplit());
						dst_item.Tag = src_item;
						ret_data.Items.Add(dst_item);
						break;
					}

				}

				return ret_data;
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
				foreach (var v in Controls)
				{
					if (!v.Verification()) { return false; }
				}
				return true;
			}
		}
	}
}
