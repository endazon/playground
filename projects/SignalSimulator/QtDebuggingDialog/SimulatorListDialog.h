#pragma once

#include <QtWidgets/QWidget>
#include "ui_SimulatorListDialog.h"

class SimulatorListDialog : public QWidget
{
    Q_OBJECT

public:
    SimulatorListDialog(QWidget *parent = Q_NULLPTR);

private:
    Ui::SimulatorListDialogClass ui;
};
