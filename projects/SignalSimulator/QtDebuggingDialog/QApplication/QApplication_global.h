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

class QApplicationForDLL : public IQApplication
{
private:
	using __MySelfType = QApplicationForDLL;

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
	//QApplicationForDLL() noexcept = delete;
	QApplicationForDLL(const __MySelfType&) noexcept = delete;
	QApplicationForDLL(__MySelfType&&) noexcept = delete;
	constexpr ~QApplicationForDLL() noexcept = default;
	//**********************************************************
	QApplicationForDLL(int argc, char* argv[]) noexcept:_ap(InstanceCreation(argc, argv))
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
