// DebuggingConsole.cpp : アプリケーションのエントリ ポイントを定義します。
//

#include "DebuggingConsole.h"

template<class __ValueType>
class TimePrintNumericEntity
{
private:
	using __MySelfType = TimePrintNumericEntity;
	__ValueType _Value;

protected:
	template<class T>
	constexpr auto CAST(T&& v) const { return static_cast<__ValueType>(v); }

	inline __ValueType GetValue() const noexcept
	{
		return _Value;
	}
	inline void SetValue(const __ValueType& v) & noexcept
	{
		_Value = v;

		auto print = [](const __ValueType v)
		{
			time_t now = time(nullptr);
			struct tm local_time;

			localtime_s(&local_time, &now);
			std::cout << "[";
			std::cout << local_time.tm_year + 1900 << "-";
			std::cout << local_time.tm_mon + 1     << "-";
			std::cout << local_time.tm_mday        << "T";
			std::cout << local_time.tm_hour        << ":";
			std::cout << local_time.tm_min         << ":";
			std::cout << local_time.tm_sec         << ":";
			std::cout << local_time.tm_isdst;
			std::cout << "]:";
			std::cout << static_cast<int>(v);
			std::cout << std::endl;
		};
		print(_Value);
	}
	inline void SetValue(const __ValueType&& v) & noexcept
	{
		SetValue(v);
	}

public:
	//**********************************************************
	//暗黙的に宣言される
	TimePrintNumericEntity() noexcept = delete;
	//TimePrintNumericEntity(const __MySelfType&) noexcept = delete;
	//TimePrintNumericEntity(__MySelfType&&) noexcept = delete;
	constexpr ~TimePrintNumericEntity() noexcept = default;
	//**********************************************************
	template<class T> constexpr TimePrintNumericEntity(const T& other) noexcept : _Value(CAST(other))
	{}
	template<class T> constexpr TimePrintNumericEntity(const T&& other) noexcept : __MySelfType(other)
	{}

	//キャスト演算子(Cast)
	inline explicit operator __ValueType() const noexcept
	{
		return GetValue();
	}
};
template<class __ValueType>
using TimePrintNumeral = UnitOfNumber::BaseNumeral<TimePrintNumericEntity<__ValueType>, __ValueType>;

template<class __ValueType>
class SpecificNumeral : public UnitOfNumber::BaseSpecificNumeral<__ValueType, SpecificNumeral<__ValueType>>
{
private:
	using __MySelfType = SpecificNumeral;
	using __InheritanceType = UnitOfNumber::BaseSpecificNumeral<__ValueType, SpecificNumeral<__ValueType>>;

protected:
	void SetValue(const __ValueType& v) & noexcept override
	{
		auto print = [](const __ValueType v)
		{
			time_t now = time(nullptr);
			struct tm local_time;

			localtime_s(&local_time, &now);
			std::cout << "[";
			std::cout << local_time.tm_year + 1900 << "-";
			std::cout << local_time.tm_mon + 1     << "-";
			std::cout << local_time.tm_mday        << "T";
			std::cout << local_time.tm_hour        << ":";
			std::cout << local_time.tm_min         << ":";
			std::cout << local_time.tm_sec         << ":";
			std::cout << local_time.tm_isdst;
			std::cout << "]:";
			std::cout << static_cast<int>(v);
			std::cout << std::endl;
		};
		print(v);

		__InheritanceType::SetValue(v);
	}

public:
	//**********************************************************
	//暗黙的に宣言される
	//SpecificNumeral() noexcept = delete;
	//SpecificNumeral(const __MySelfType&) noexcept = delete;
	//SpecificNumeral(__MySelfType&&) noexcept = delete;
	constexpr ~SpecificNumeral() noexcept = default;
	//**********************************************************
	constexpr SpecificNumeral(const __ValueType & init) noexcept : __InheritanceType(init)
	{}
	constexpr SpecificNumeral(const __ValueType && init = 0) noexcept : __MySelfType(init)
	{}
	template<class T> constexpr SpecificNumeral(const T & other) noexcept : __MySelfType(this->CAST(other))
	{}
	template<class T> constexpr SpecificNumeral(const T && other) noexcept : __MySelfType(other)
	{}

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

int main()
{
	auto timer = []()
	{
		//TimePrintNumeral<int> time = 0;
		SpecificNumeral<int> time = 0;
		// QueryPerformanceCounter関数の1秒当たりのカウント数を取得する
		LARGE_INTEGER freq;
		QueryPerformanceFrequency(&freq);

		LARGE_INTEGER start, end;

		while (true)
		{
			{
				GeneralPurposeTimer::MeasuringElapsedTime<GeneralPurposeTimer::TimerAccessor::HighPrecision, GeneralPurposeTimer::TimeUnits::nanoseconds> elapsedTime;
				QueryPerformanceCounter(&start);
				QueryPerformanceCounter(&end);
				for (; static_cast<double>(end.QuadPart - start.QuadPart) * 1000.0 / freq.QuadPart < 1000; QueryPerformanceCounter(&end));
				time++;
			}
		}

	};
	std::thread thread(timer);
	thread.join();
	return 0;
}
