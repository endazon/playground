using CommandGenerator.Utility.InputDisplay.Adapter;
using System;

namespace CommandGenerator.Utility.InputDisplay.Facade
{
    class Searcher
    {
        public IInput getInputDisplayOfName(string name)
        {
			switch (name)
			{
                case "InputFile"        : return new InputFile();
                case "InputSelection"   : return new InputSelection();
                case "InputDecimal"     : return new InputDecimal();
                case "InputHexadecimal" : return new InputHexadecimal();
                case "InputString"      : return new InputString();
                default                 : throw  new Exception();
			}
        }

        public IInput getInputDisplayOfType(string type)
        {
			switch (type)
			{
				case "FILE_SELECT"  : return new InputFile();
				case "SELECT"       : return new InputSelection();
                case "DEC"          : return new InputDecimal();
                case "HEX"          : return new InputHexadecimal();
                case "ASCII"        : return new InputString();
                default             : throw  new Exception();
			}
		}
    }
}
