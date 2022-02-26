#pragma once

class IQApplication
{
public:
    //Qt
    virtual int exec() = 0;
};

#if defined(SIMULATORLISTDIALOG_LIBRARY)

#  define SIMULATORLISTDIALOG_EXPORT extern "C" _declspec(dllexport)

#else
#include "Utility.hpp"

#  define SIMULATORLISTDIALOG_EXPORT extern "C" __declspec(dllimport)

class QApplicationOfDLL : public IQApplication
{
private:
	using __MySelfType = QApplicationOfDLL;

	using load_QApplication_symbol = IQApplication* (*)(int argc, char* argv[]);

	IQApplication* _ap;

	inline Utility::DLLLoader& DLL()
	{
		return Utility::DLLLoader::GetInstance("QApplication.dll");
	}
	inline IQApplication* InstanceCreationForQApplication(int argc, char* argv[])
	{
		return DLL().GetFunction<load_QApplication_symbol>("load_QApplication_symbol")(argc, argv);
	}

public:
	//**********************************************************
	//ˆÃ–Ù“I‚ÉéŒ¾‚³‚ê‚é
	//QApplicationOfDLL() noexcept = delete;
	QApplicationOfDLL(const __MySelfType&) noexcept = delete;
	QApplicationOfDLL(__MySelfType&&) noexcept = delete;
	constexpr ~QApplicationOfDLL() noexcept = default;
	//**********************************************************
	QApplicationOfDLL(int argc, char* argv[]) noexcept:_ap(InstanceCreationForQApplication(argc, argv))
	{}

	//‘ã“ü‰‰ŽZŽq(Assignment)
	//**********************************************************
	//ˆÃ–Ù“I‚ÉéŒ¾‚³‚ê‚é
	__MySelfType& operator=(const __MySelfType&) noexcept = delete;
	__MySelfType& operator=(__MySelfType&&) &noexcept = delete;
	//**********************************************************

	//Qt
	int exec() override
	{
		return _ap->exec();
	}
};

#endif
