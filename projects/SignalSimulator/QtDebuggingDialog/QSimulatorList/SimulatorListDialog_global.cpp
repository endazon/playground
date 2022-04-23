#include "SimulatorListDialog.h"
#include "SimulatorListDialog_global.h"

namespace {

//SimulatorListDialog
// DLLインターフェースクラスの実装
class SimulatorListDialogForDLL : public ISimulatorListDialog
{
public:
    SimulatorListDialogForDLL()
    : dialog(Q_NULLPTR)
    {}

    //Qt
    bool isVisible() override
    {
        return dialog.isVisible();
    }
    bool isHidden() override
    {
        return dialog.isHidden();
    }
    void show() override
    {
        dialog.show();
    }
    void showMaximized() override
    {
        dialog.showMaximized();
    }
    void close() override
    {
        dialog.close();
    }

    void AddElement(long long key, std::string Name, std::string Group, std::string Comment, long double Value) override
    {
        dialog.AddElement(key, Name, Group, Comment, Value);
    }

    void RemovalElement(long long key) override
    {
        dialog.RemovalElement(key);
    }

    void ValueUpdate(long long key, long double Value) override
    {
        dialog.ValueUpdate(key, Value);
    }

private:
    SimulatorListDialog dialog;
};

// エクスポート関数の実装
SIMULATORLISTDIALOG_EXPORT SimulatorListDialogForDLL* InstanceCreation()
{
    return new SimulatorListDialogForDLL();
}

SIMULATORLISTDIALOG_EXPORT void InstanceDestroyed(SimulatorListDialogForDLL* p)
{
    delete p;
}

}