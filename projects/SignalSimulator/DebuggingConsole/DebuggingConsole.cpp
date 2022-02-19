// DebuggingConsole.cpp : アプリケーションのエントリ ポイントを定義します。
//

#include "DebuggingConsole.h"

using namespace Simulator;

class Signal : public BaseSimulator
{
private:
	using __MySelfType = Signal;
	using __InheritanceType = BaseSimulator;

protected:
	void Changing() noexcept override
	{
	
	}

	void Changed() noexcept override
	{
		//DebuggingTimestampForChanged();
		//std::cout << to_string() << std::endl;
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

class TimeSimulateTest : public BaseTimeSimulator
{
private:
	using __MySelfType = TimeSimulateTest;
	using __InheritanceType = BaseTimeSimulator;

protected:
	inline void Changing() noexcept override{}
	inline void Changed()  noexcept override{}

	inline void UpdateIn100usCycle() noexcept override{}
	inline void UpdateIn200usCycle() noexcept override{}
	inline void UpdateIn500usCycle() noexcept override{}
	inline void UpdateIn1msCycle()   noexcept override{}
	inline void UpdateIn2msCycle()   noexcept override{}
	inline void UpdateIn5msCycle()   noexcept override{}
	inline void UpdateIn10msCycle()  noexcept override{}
	inline void UpdateIn20msCycle()  noexcept override{}
	inline void UpdateIn50msCycle()  noexcept override{}
	inline void UpdateIn100msCycle() noexcept override{}
	inline void UpdateIn200msCycle() noexcept override{}
	inline void UpdateIn500msCycle() noexcept override{}
	inline void UpdateIn1000msCycle()noexcept override{}

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

//static TimeSimulateTest timeSimulate = { 0,"Test0", "テスト", "☆★☆彡" };
//static TimeSimulateTest timeSimulate[5000] = 
//{ 
//	{0,"Test0", "テスト", "☆★☆彡"},
//	{1,"Test1", "テスト", "☆★☆彡"}
//};

int main()
{
	auto timer = []()
	{
		GeneralPurposeTimer::Measurement::ElapsedTimeDetection<GeneralPurposeTimer::Measurement::MediumPrecision> timer = 1000 * 1000;
		Signal time(1, "Test", "テスト", "☆★☆彡");

		while (true)
		{
			if (timer.isElapsed() )
			{
				time++;
			}
		}

	};

	std::thread thread(timer);
	thread.join();

	return 0;
}
