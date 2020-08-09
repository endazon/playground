using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace CommandGenerator.Target.Common.Range
{
	class Range<T> 
	{
		#region Property
		public T Maximum { set; get; }
		public T Minimum { set; get; }
		#endregion

		#region Constructor
		public Range()
		{

		}
		public Range(T maximum) : this()
		{
			Maximum = maximum;
		}
		public Range(T maximum, T minimum) : this(maximum)
		{
			Minimum = minimum;
		}
		#endregion
	}
}
