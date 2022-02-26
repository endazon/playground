#pragma once

#include <windows.h>
#include <stdint.h>
#include <type_traits>
#include <cassert>

namespace Utility
{
	class DLLLoader
	{
	private:
		using __MySelfType = DLLLoader;

		HMODULE _hModule;

		//**********************************************************
		//à√ñŸìIÇ…êÈåæÇ≥ÇÍÇÈ
		//DLLLoader() noexcept = delete;
		DLLLoader(const __MySelfType&) noexcept = delete;
		DLLLoader(__MySelfType&&) noexcept = delete;
		//constexpr ~DLLLoader() noexcept = default;
		//**********************************************************
		DLLLoader(LPCSTR name) noexcept : _hModule(LoadLibraryA(name))
		{
			assert(_hModule != nullptr);
		}

		DLLLoader(LPCWSTR name) noexcept : _hModule(LoadLibraryW(name))
		{
			assert(_hModule != nullptr);
		}

		~DLLLoader() noexcept
		{
			FreeLibrary(_hModule);
		}

		//ë„ì¸ââéZéq(Assignment)
		//**********************************************************
		//à√ñŸìIÇ…êÈåæÇ≥ÇÍÇÈ
		__MySelfType& operator=(const __MySelfType&) noexcept = delete;
		__MySelfType& operator=(__MySelfType&&) &noexcept = delete;
		//**********************************************************

		inline HMODULE& GetModule()
		{
			return _hModule;
		}

	public:
		inline static __MySelfType& GetInstance(LPCSTR name)
		{
			static __MySelfType instance(name);
			return instance;
		}

		inline static __MySelfType& GetInstance(LPCWSTR name)
		{
			static __MySelfType instance(name);
			return instance;
		}

		template<class T> inline T GetFunction(LPCSTR funcName)
		{
			return reinterpret_cast<T>(GetProcAddress(GetModule(), funcName));
		}
	};
}