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

//static TimeSimulateTest timeSimulate = { 0,"Test0", "テスト", "☆★☆彡" };
//static TimeSimulateTest timeSimulate[5000] = 
//{ 
//	{0,"Test0", "テスト", "☆★☆彡"},
//	{1,"Test1", "テスト", "☆★☆彡"}
//};

class SimulatorListDialogOfDLL : public ISimulatorListDialog
{
private:
	using __MySelfType = SimulatorListDialogOfDLL;

	ISimulatorListDialog* _dialog;

	inline Utility::DLLLoader& DLL()
	{
		return Utility::DLLLoader::GetInstance("SimulatorListDialog.dll");
	}
	inline ISimulatorListDialog* InstanceCreationForSimulatorListDialog()
	{
		return DLL().GetFunction<load_SimulatorListDialog_symbol>("load_SimulatorListDialog_symbol")();
	}

	inline void InstanceDestroyedForSimulatorListDialog(ISimulatorListDialog* p)
	{
		DLL().GetFunction<destroy_SimulatorListDialog_symbol>("destroy_SimulatorListDialog_symbol")(p);
	}

public:
	//**********************************************************
	//暗黙的に宣言される
	//SimulatorListDialogOfDLL() noexcept = delete;
	SimulatorListDialogOfDLL(const __MySelfType&) noexcept = delete;
	SimulatorListDialogOfDLL(__MySelfType&&) noexcept = delete;
	//constexpr ~SimulatorListDialogOfDLL() noexcept = default;
	//**********************************************************
	SimulatorListDialogOfDLL() noexcept:_dialog(InstanceCreationForSimulatorListDialog())
	{}

	~SimulatorListDialogOfDLL() noexcept
	{
		InstanceDestroyedForSimulatorListDialog(_dialog);
	}

	//代入演算子(Assignment)
	//**********************************************************
	//暗黙的に宣言される
	__MySelfType& operator=(const __MySelfType&) noexcept = delete;
	__MySelfType& operator=(__MySelfType&&) &noexcept = delete;
	//**********************************************************

	//Qt
	bool isVisible() override
	{
		return _dialog->isVisible();
	}
	bool isHidden() override
	{
		return _dialog->isHidden();
	}
	void show() override
	{
		_dialog->show();
	}
	void showMaximized() override
	{
		_dialog->showMaximized();
	}
	void close() override
	{
		_dialog->close();
	}

	void AddElement(long long key, std::string Name, std::string Group, std::string Comment, long double Value) override
	{
		_dialog->AddElement(key, Name, Group, Comment, Value);
	}

	void RemovalElement(long long key) override
	{
		_dialog->RemovalElement(key);
	}

	void ValueUpdate(long long key, long double Value) override
	{
		_dialog->ValueUpdate(key, Value);
	}
};

int main()
{
	auto timer = []()
	{
		GeneralPurposeTimer::Measurement::ElapsedTimeDetection<GeneralPurposeTimer::Measurement::MediumPrecision> timer = 1000 * 1000;
		Signal time(1, "Test", "テスト", "☆★☆彡");
		SimulatorListDialogOfDLL dialog;

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
