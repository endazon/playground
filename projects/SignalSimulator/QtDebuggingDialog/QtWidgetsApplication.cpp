#include <windows.h>
#include "QtWidgetsApplication.h"

QtWidgetsApplication::QtWidgetsApplication(QWidget *parent)
: QMainWindow(parent)
{
    ui.setupUi(this);
    pSimulatorListDialog = new SimulatorListDialog();
}

QtWidgetsApplication::~QtWidgetsApplication()
{
    delete pSimulatorListDialog;
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