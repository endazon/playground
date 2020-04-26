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
			public string Name { get; set; } = "";
			public string Command { get; set; } = "";
		}

		public class CommandCsvObject
		{
			public string Name { get; set; } = "";
			public string Version { get; set; } = "";

			public List<Detail> Items { get; set; } = new List<Detail>();
		}
	}
}
