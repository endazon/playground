// DebuggingConsole.cpp : アプリケーションのエントリ ポイントを定義します。
//

#include "DebuggingConsole.h"

class Signal : public Simulator::BaseSimulator
{
private:
	using __MySelfType = Signal;
	using __InheritanceType = BaseSimulator;

protected:
	//オーバーライド
	void SetValue(const __ValueType& v) & noexcept override final
	{
		__InheritanceType::SetValue(v);

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

class TimeSimulateTest : public Simulator::BaseTimeSimulator
{
private:
	using __MySelfType = TimeSimulateTest;
	using __InheritanceType = BaseTimeSimulator;

protected:
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

class SignalInstanceUpdateFunction : public Simulator::BaseSimulator::ISignalInstanceUpdateFunction
{
public:
	SignalInstanceUpdateFunction(SimulatorListDialogForDLL& d) :dialog(d) {}

	static SignalInstanceUpdateFunction* GetInstance(SimulatorListDialogForDLL& dialog)
	{
		static SignalInstanceUpdateFunction instance(dialog);
		return &instance;
	}

	void Registered(Simulator::BaseSimulator& rSignalInstance) override
	{
		dialog.AddElement(
			reinterpret_cast<long long>(&rSignalInstance),
			std::string(rSignalInstance.Name().data(), rSignalInstance.Name().size()),
			std::string(rSignalInstance.Group().data(), rSignalInstance.Group().size()),
			std::string(rSignalInstance.Comment().data(), rSignalInstance.Comment().size()),
			static_cast<Simulator::BaseSimulator::__ValueType>(rSignalInstance)
		);
	}

	void Delete(Simulator::BaseSimulator& rSignalInstance) override
	{
		dialog.RemovalElement(reinterpret_cast<long long>(&rSignalInstance));
	}

	void ValueUpdate(Simulator::BaseSimulator& rSignalInstance) override
	{
		dialog.ValueUpdate(
			reinterpret_cast<long long>(&rSignalInstance),
			static_cast<Simulator::BaseSimulator::__ValueType>(rSignalInstance)
		);
	}

private:
	SimulatorListDialogForDLL& dialog;
};

//static TimeSimulateTest timeSimulate = { 0,"Test0", "テスト", "☆★☆彡" };
//static TimeSimulateTest timeSimulate[5000] = 
//{ 
//	{0,"Test0", "テスト", "☆★☆彡"},
//	{1,"Test1", "テスト", "☆★☆彡"}
//};

int main(int argc, char* argv[])
{
	QApplicationForDLL ap(argc, argv);
	SimulatorListDialogForDLL dialog;

	Simulator::BaseSimulator::RegisterSignalListAcquisitionFunction(SignalInstanceUpdateFunction::GetInstance(dialog));
	dialog.show();
	auto timer = []()
	{
		GeneralPurposeTimer::Measurement::ElapsedTimeDetection<GeneralPurposeTimer::Measurement::MediumPrecision> timer = 1000 * 1000;
		Signal time(1, "Test", "テスト", "☆★☆彡");
		Signal* temporary = nullptr;

		while (true)
		{
			if (timer.isElapsed() )
			{
				time++;

				if (temporary == nullptr) {
					temporary = new Signal(time * 2, "temporary", "TimeSimulateTest");
				}
				else {
					delete temporary;
					temporary = nullptr;
				}
			}
		}

	};

	std::thread thread(timer);
	thread.detach();

	return ap.exec();
}
