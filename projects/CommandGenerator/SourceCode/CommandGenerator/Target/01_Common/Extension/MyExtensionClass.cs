using System;
using System.Collections.Generic;

namespace Extension
{
	public static class MyExtensionClass
	{
		/// <summary>
		/// stringの文字列を指定した数で分割する。
		/// </summary>
		public static List<string> NumStringSplit(this string value, int count = 2)
		{
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
	}
}
