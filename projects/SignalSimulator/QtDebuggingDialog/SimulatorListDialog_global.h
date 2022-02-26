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
#  define SIMULATORLISTDIALOG_EXPORT extern "C" __declspec(dllimport)

using load_SimulatorListDialog_symbol = ISimulatorListDialog*(*)();
using destroy_SimulatorListDialog_symbol = void(*)(ISimulatorListDialog*);

#endif
