#pragma once

#include <QtWidgets/QMainWindow>
#include "ui_QtWidgetsApplication.h"
#include "SimulatorListDialog_global.h"

class QtWidgetsApplication : public QMainWindow
{
    Q_OBJECT

public:
    QtWidgetsApplication(QWidget *parent = Q_NULLPTR);
    ~QtWidgetsApplication();

public slots:
    void ShowSimulatorListDialog();
    void CloseSimulatorListDialog();

protected:
    inline SimulatorListDialog* InstanceCreationForSimulatorListDialog();
    inline void InstanceDestroyedForSimulatorListDialog(SimulatorListDialog* p);

private:
    Ui::QtWidgetsApplicationClass ui;
    HMODULE hModuleSimulatorListDialog;
    SimulatorListDialog* pSimulatorListDialog;
};
