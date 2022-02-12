#pragma once

#include "UnitOfNumber.hpp"
#include "SignalsCustom.hpp"

using namespace UnitOfNumber;
namespace Signals {
	template<class __ValueType>
	class Signal : public BaseSpecificNumeral<__ValueType, Numeral<__ValueType>>
	{
	private:
		using __MySelfType = Signal;
		using __InheritanceType = BaseSpecificNumeral<__ValueType, Numeral<__ValueType>>;

		const std::string _name;
		const std::string _group;
		const std::string _comment;

	protected:
		virtual void Changing() & noexcept {}
		virtual void Changed() & noexcept {}

		void DebuggingTimestamp() noexcept
		{
			GeneralPurposeTimer::DateFormat::UTC date;
			std::cout << "【Name:" << name() << "-Group:" << group() << "-Comment:" << comment() << "】";
			std::cout << date.format() << ":";
		}
		void DebuggingTimestampForChanged() noexcept
		{
			DebuggingTimestamp();
			std::cout << this->GetValue() << std::endl;
		}

		void SetValue(const __ValueType& v) & noexcept override
		{
			Changing();
			__InheritanceType::SetValue(v);
			Changed();

			//DebuggingTimestampForChanged();
		}

	public:
		//using __InheritanceType::__InheritanceType; //継承元のコンストラクタは使わない
		//**********************************************************
		//Signal宣言される
		//Signal() noexcept = delete;
		//Signal(const __MySelfType&) noexcept = delete;
		//Signal(__MySelfType&&) noexcept = delete;
		//constexpr ~Signal() noexcept = default;
		//**********************************************************
		constexpr Signal(const __ValueType& init, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __InheritanceType(init), _name(name), _group(group), _comment(comment)
		{
			SignalCustom<__MySelfType>::registeredSignalObjects(this);
		}
		constexpr Signal(const __ValueType&& init = 0, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __MySelfType(init, name, group, comment)
		{}
		template<class T> constexpr Signal(const T& other, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __MySelfType(this->CAST(other), name, group, comment)
		{}
		template<class T> constexpr Signal(const T&& other, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __MySelfType(other, name, group, comment)
		{}
		constexpr ~Signal() noexcept
		{
			SignalCustom<__MySelfType>::deleteSignalObject(this);
		}

		//代入演算子(Assignment)
		//**********************************************************
		//暗黙的に宣言される
		//__MySelfType& operator=(const __MySelfType&) noexcept = delete;
		//__MySelfType& operator=(__MySelfType&&) & noexcept = delete;
		//**********************************************************
		template<class T> inline __MySelfType& operator=(T& rhs) noexcept
		{
			__InheritanceType::operator=(rhs);
			return *this;
		}
		template<class T> inline __MySelfType& operator=(T&& rhs) & noexcept
		{
			return operator=(rhs);
		}

		inline const std::string_view name()    noexcept { return _name;    }
		inline const std::string_view group()   noexcept { return _group;   }
		inline const std::string_view comment() noexcept { return _comment; }
	};
}