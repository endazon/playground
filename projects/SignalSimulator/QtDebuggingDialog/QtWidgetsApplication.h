#pragma once

#include <QtWidgets/QMainWindow>
#include "ui_QtWidgetsApplication.h"
#include "SimulatorListDialog.h"

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
    void timerEvent(QTimerEvent* event) override;

private:
    Ui::QtWidgetsApplicationClass ui;
    SimulatorListDialog Dialog;
   
};
