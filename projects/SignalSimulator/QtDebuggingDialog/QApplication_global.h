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

	IQApplication* _ap;

	inline Utility::DLLLoader& DLL()
	{
		return Utility::DLLLoader::GetInstance("QApplication.dll");
	}
	inline IQApplication* InstanceCreation(int argc, char* argv[])
	{
		return DLL().GetFunction<IQApplication*(*)(int argc, char* argv[])>(__func__)(argc, argv);
	}

public:
	//**********************************************************
	//暗黙的に宣言される
	//QApplicationOfDLL() noexcept = delete;
	QApplicationOfDLL(const __MySelfType&) noexcept = delete;
	QApplicationOfDLL(__MySelfType&&) noexcept = delete;
	constexpr ~QApplicationOfDLL() noexcept = default;
	//**********************************************************
	QApplicationOfDLL(int argc, char* argv[]) noexcept:_ap(InstanceCreation(argc, argv))
	{}

	//代入演算子(Assignment)
	//**********************************************************
	//暗黙的に宣言される
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
