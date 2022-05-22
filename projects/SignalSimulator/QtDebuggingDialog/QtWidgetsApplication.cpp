#include <windows.h>
#include "QtWidgetsApplication.h"

QtWidgetsApplication::QtWidgetsApplication(QWidget *parent)
: QMainWindow(parent)
, Dialog(parent)
, Dialog2(parent)
{
    ui.setupUi(this);

    Dialog.AddElement(0, "time", __func__, "", 0);
    startTimer(1);
}

QtWidgetsApplication::~QtWidgetsApplication()
{}

void QtWidgetsApplication::ShowSimulatorListDialog()
{
    if (Dialog.isVisible()) { return; }
    Dialog.show();
    //Dialog.showMaximized();
}

void QtWidgetsApplication::CloseSimulatorListDialog()
{
    if (Dialog.isHidden()) { return; }
    Dialog.close();
}

void QtWidgetsApplication::ShowCommunicationHistoryListDialog()
{
    if (Dialog2.isVisible()) { return; }
    Dialog2.show();
    //Dialog2.showMaximized();
}

void QtWidgetsApplication::CloseCommunicationHistoryListDialog()
{
    if (Dialog2.isHidden()) { return; }
    Dialog2.close();
}

void QtWidgetsApplication::timerEvent(QTimerEvent* event)
{
    static int time = 0;
    Dialog.ValueUpdate(0, time++);

    switch (time%1000)
    {
    case 499:
        Dialog.AddElement(1, "time", __func__, "", 100);
        break;
    case 999:
        Dialog.RemovalElement(1);
        Dialog2.AddMessage("MSG", QString::number(time).toStdString());
        break;
    }

    QMainWindow::timerEvent(event);
}