#pragma once

#include "UnitOfNumber.hpp"
#include "SimulatorCustom.hpp"

using namespace UnitOfNumber;
namespace Simulator {
	template<class __ValueType>
	class Simulate : public BaseSpecificNumeral<__ValueType, Numeral<__ValueType>>
	{
	private:
		using __MySelfType = Simulate;
		using __InheritanceType = BaseSpecificNumeral<__ValueType, Numeral<__ValueType>>;

		const std::string _name;
		const std::string _group;
		const std::string _comment;

	protected:
		virtual void Changing() & noexcept = 0;
		virtual void Changed() & noexcept = 0;

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
		//暗黙的に宣言される
		//Simulate() noexcept = delete;
		//Simulate(const __MySelfType&) noexcept = delete;
		//Simulate(__MySelfType&&) noexcept = delete;
		//constexpr ~Simulate() noexcept = default;
		//**********************************************************
		constexpr Simulate(const __ValueType& init, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __InheritanceType(init), _name(name), _group(group), _comment(comment)
		{
			SimulateCustom<__MySelfType>::registeredSignalObjects(this);
		}
		constexpr Simulate(const __ValueType&& init = 0, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __MySelfType(init, name, group, comment)
		{}
		template<class T> constexpr Simulate(const T& other, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __MySelfType(this->CAST(other), name, group, comment)
		{}
		template<class T> constexpr Simulate(const T&& other, const std::string name = "", const std::string group = "", const std::string comment = "") noexcept : __MySelfType(other, name, group, comment)
		{}
		constexpr ~Simulate() noexcept
		{
			SimulateCustom<__MySelfType>::deleteSignalObject(this);
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

	template<class __ValueType>
	class Signal : public Simulate<__ValueType>
	{
	private:
		using __MySelfType = Signal;
		using __InheritanceType = Simulate<__ValueType>;

	protected:
		void Changing() & noexcept override
		{

		}

		void Changed() & noexcept override
		{
			this->DebuggingTimestampForChanged();
		}

	public:
		using __InheritanceType::__InheritanceType; //継承元のコンストラクタは使わない


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
	};
}