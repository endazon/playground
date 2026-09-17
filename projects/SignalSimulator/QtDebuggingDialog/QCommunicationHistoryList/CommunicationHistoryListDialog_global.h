//#pragma once
//
////struct SimulatorListAttribute
////{
////	std::string Name;
////	std::string Group;
////	std::string Comment;
////	long double Value;
////};
////
////struct SimulatorListElement
////{
////	long long key;
////	SimulatorListAttribute& attribute;
////};
//
//class ISimulatorListDialog
//{
//public:
//    //Qt
//    virtual bool isVisible() = 0;
//    virtual bool isHidden() = 0;
//    virtual void show() = 0;
//    virtual void showMaximized() = 0;
//    virtual void close() = 0;
//
//    virtual void AddElement(long long key, std::string Name, std::string Group, std::string Comment, long double Value) = 0;
//    virtual void RemovalElement(long long key) = 0;
//    virtual void ValueUpdate(long long key, long double Value) = 0;
//};
//
//#if defined(COMMUNICATIONHISTORYLISTDIALOG_LIBRARY)
//
//#  define SIMULATORLISTDIALOG_EXPORT extern "C" _declspec(dllexport)
//
//#else
//#include "Utility.hpp"
//
//#  define SIMULATORLISTDIALOG_EXPORT extern "C" __declspec(dllimport)
//
//class SimulatorListDialogForDLL : public ISimulatorListDialog
//{
//private:
//	using __MySelfType = SimulatorListDialogForDLL;
//
//	ISimulatorListDialog* _dialog;
//
//	inline Utility::DLLLoader& DLL()
//	{
//		return Utility::DLLLoader::GetInstance("SimulatorListDialog.dll");
//	}
//	inline ISimulatorListDialog* InstanceCreation()
//	{
//		return DLL().GetFunction<ISimulatorListDialog* (*)()>(__func__)();
//	}
//
//	inline void InstanceDestroyed(ISimulatorListDialog* p)
//	{
//		DLL().GetFunction<void(*)(ISimulatorListDialog*)>(__func__)(p);
//	}
//
//public:
//	//**********************************************************
//	//暗黙的に宣言される
//	//SimulatorListDialogForDLL() noexcept = delete;
//	SimulatorListDialogForDLL(const __MySelfType&) noexcept = delete;
//	SimulatorListDialogForDLL(__MySelfType&&) noexcept = delete;
//	//constexpr ~SimulatorListDialogForDLL() noexcept = default;
//	//**********************************************************
//	SimulatorListDialogForDLL() noexcept:_dialog(InstanceCreation())
//	{}
//
//	~SimulatorListDialogForDLL() noexcept
//	{
//		InstanceDestroyed(_dialog);
//	}
//
//	//代入演算子(Assignment)
//	//**********************************************************
//	//暗黙的に宣言される
//	__MySelfType& operator=(const __MySelfType&) noexcept = delete;
//	__MySelfType& operator=(__MySelfType&&) &noexcept = delete;
//	//**********************************************************
//
//	//Qt
//	bool isVisible() override
//	{
//		return _dialog->isVisible();
//	}
//	bool isHidden() override
//	{
//		return _dialog->isHidden();
//	}
//	void show() override
//	{
//		_dialog->show();
//	}
//	void showMaximized() override
//	{
//		_dialog->showMaximized();
//	}
//	void close() override
//	{
//		_dialog->close();
//	}
//
//	void AddElement(long long key, std::string Name, std::string Group, std::string Comment, long double Value) override
//	{
//		_dialog->AddElement(key, Name, Group, Comment, Value);
//	}
//
//	void RemovalElement(long long key) override
//	{
//		_dialog->RemovalElement(key);
//	}
//
//	void ValueUpdate(long long key, long double Value) override
//	{
//		_dialog->ValueUpdate(key, Value);
//	}
//};
//
//#endif
