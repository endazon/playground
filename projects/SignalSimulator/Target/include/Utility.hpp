#pragma once

#include <windows.h>
#include <stdint.h>
#include <type_traits>
#include <cassert>
#include <codecvt>
#include <map>

namespace Utility
{
	class DLLLoader
	{
	private:
		using __MySelfType = DLLLoader;

		static inline std::map<std::wstring, __MySelfType*> _ListOfEntities;
		HMODULE _hModule;

		//**********************************************************
		//暗黙的に宣言される
		//DLLLoader() noexcept = delete;
		DLLLoader(const __MySelfType&) noexcept = delete;
		DLLLoader(__MySelfType&&) noexcept = delete;
		//constexpr ~DLLLoader() noexcept = default;
		//**********************************************************
		DLLLoader(std::string name) noexcept : _hModule(LoadLibraryA(name.c_str()))
		{
			assert(_hModule != nullptr);
		}

		DLLLoader(std::wstring name) noexcept : _hModule(LoadLibraryW(name.c_str()))
		{
			assert(_hModule != nullptr);
		}

		~DLLLoader() noexcept
		{
			FreeLibrary(_hModule);
		}

		//代入演算子(Assignment)
		//**********************************************************
		//暗黙的に宣言される
		__MySelfType& operator=(const __MySelfType&) noexcept = delete;
		__MySelfType& operator=(__MySelfType&&) &noexcept = delete;
		//**********************************************************

		inline HMODULE& GetModule()
		{
			return _hModule;
		}

		static __MySelfType& Instance(std::wstring name)
		{
			//まだロードしていない
			if (_ListOfEntities.find(name) == _ListOfEntities.end())
			{
				_ListOfEntities[name] = new __MySelfType(name.c_str());
			}
			return *_ListOfEntities[name];
		}

		//文字列変換(UTF-8→UTF-16)
		//※C++17では非推奨
		static std::wstring convertStringToWString(const std::string& from)
		{
			//UTF-8とUTF-16の相互変換を行うコンバーター
			std::wstring_convert<std::codecvt_utf8<std::wstring::value_type>, std::wstring::value_type> converter;

			//UTF-8からUTF-16に変換
			return converter.from_bytes(from);			
		}

	public:
		inline static __MySelfType& GetInstance(std::string name)
		{
			return Instance(convertStringToWString(name));
		}

		inline static __MySelfType& GetInstance(std::wstring name)
		{
			return Instance(name);
		}

		template<class T> inline T GetFunction(std::string funcName)
		{
			return reinterpret_cast<T>(GetProcAddress(GetModule(), funcName.c_str()));
		}
	};
}