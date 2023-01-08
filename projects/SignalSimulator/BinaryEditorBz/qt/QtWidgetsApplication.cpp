#include "QtWidgetsApplication.h"
#include "stdafx.h"

QtWidgetsApplication::QtWidgetsApplication(QWidget *parent)
    : QMainWindow(parent)
    , viewer(parent)
{
    ui.setupUi(this);
}

void QtWidgetsApplication::show()
{
    viewer.show();
    QMainWindow::show();
}