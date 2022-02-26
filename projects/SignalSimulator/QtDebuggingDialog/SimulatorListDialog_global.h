#pragma once

class ISimulatorListDialog
{
public:
    //Qt
    virtual bool isVisible() = 0;
    virtual bool isHidden() = 0;
    virtual void show() = 0;
    virtual void showMaximized() = 0;
    virtual void close() = 0;

    virtual void AddElement(long long key, std::string Name, std::string Group, std::string Comment, long double Value) = 0;
    virtual void RemovalElement(long long key) = 0;
    virtual void ValueUpdate(long long key, long double Value) = 0;
};

#if defined(SIMULATORLISTDIALOG_LIBRARY)

#  define SIMULATORLISTDIALOG_EXPORT extern "C" _declspec(dllexport)

#else
#include "Utility.hpp"

#  define SIMULATORLISTDIALOG_EXPORT extern "C" __declspec(dllimport)

class SimulatorListDialogOfDLL : public ISimulatorListDialog
{
private:
	using __MySelfType = SimulatorListDialogOfDLL;

	using load_SimulatorListDialog_symbol = ISimulatorListDialog * (*)();
	using destroy_SimulatorListDialog_symbol = void(*)(ISimulatorListDialog*);

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
	//ˆÃ–Ù“I‚ÉéŒ¾‚³‚ê‚é
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

	//‘ã“ü‰‰ŽZŽq(Assignment)
	//**********************************************************
	//ˆÃ–Ù“I‚ÉéŒ¾‚³‚ê‚é
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

#endif
