// DebuggingConsole.cpp : アプリケーションのエントリ ポイントを定義します。
//

#include "DebuggingConsole.h"

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
			GeneralPurposeTimer::DateFormat::UTC date;
			auto format = date.format();

			std::cout << format << ":" << static_cast<int>(v) << std::endl;
		};
		print(v);

		__InheritanceType::SetValue(v);
	}

public:
	using __InheritanceType::__InheritanceType;

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
		SpecificNumeral<int> time = 0;
		// QueryPerformanceCounter関数の1秒当たりのカウント数を取得する
		LARGE_INTEGER freq;
		QueryPerformanceFrequency(&freq);

		LARGE_INTEGER start, end;

		while (true)
		{
			{
				//GeneralPurposeTimer::Measurement::MeasuringElapsedTime<GeneralPurposeTimer::Measurement::LowPrecision>    elapsedTime1;
				//GeneralPurposeTimer::Measurement::MeasuringElapsedTime<GeneralPurposeTimer::Measurement::MediumPrecision> elapsedTime2;
				//GeneralPurposeTimer::Measurement::MeasuringElapsedTime<GeneralPurposeTimer::Measurement::HighPrecision>   elapsedTime3;
				QueryPerformanceCounter(&start);
				QueryPerformanceCounter(&end);
				for (; static_cast<double>(end.QuadPart - start.QuadPart) * 1000.0 / freq.QuadPart < 200; QueryPerformanceCounter(&end));
				time++;
			}
		}

	};

	std::thread thread(timer);
	thread.join();

	return 0;
}
