#include <windows.h>
#include "QtWidgetsApplication.h"

QtWidgetsApplication::QtWidgetsApplication(QWidget *parent)
: QMainWindow(parent)
//, hModuleSimulatorListDialog(nullptr)                                   //静的リンク時
, hModuleSimulatorListDialog(LoadLibrary(L"SimulatorListDialog.dll"))   //動的リンク時
, pSimulatorListDialog(InstanceCreationForSimulatorListDialog())
{
    ui.setupUi(this);
}

QtWidgetsApplication::~QtWidgetsApplication()
{
    InstanceDestroyedForSimulatorListDialog(pSimulatorListDialog);
    FreeLibrary(hModuleSimulatorListDialog);
}

SimulatorListDialog* QtWidgetsApplication::InstanceCreationForSimulatorListDialog()
{
    if (hModuleSimulatorListDialog != nullptr) { return reinterpret_cast<load_SimulatorListDialog_symbol>(GetProcAddress(hModuleSimulatorListDialog, "load_SimulatorListDialog_symbol"))(); }
    return new SimulatorListDialog();
}

void QtWidgetsApplication::InstanceDestroyedForSimulatorListDialog(SimulatorListDialog* p)
{
    if (hModuleSimulatorListDialog != nullptr) { reinterpret_cast<destroy_SimulatorListDialog_symbol>(GetProcAddress(hModuleSimulatorListDialog, "destroy_SimulatorListDialog_symbol"))(p); }
    delete p;
}

void QtWidgetsApplication::ShowSimulatorListDialog()
{
    if (pSimulatorListDialog->isVisible()) { return; }
    pSimulatorListDialog->show();
    //pSimulatorListDialog->showMaximized();
}

void QtWidgetsApplication::CloseSimulatorListDialog()
{
    if (pSimulatorListDialog->isHidden()) { return; }
    pSimulatorListDialog->close();
}