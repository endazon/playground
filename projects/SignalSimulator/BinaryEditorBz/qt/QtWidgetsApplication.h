#pragma once

#include <QtWidgets/QMainWindow>
#include "ui_QtWidgetsApplication.h"
#include "Viewer.h"

class QtWidgetsApplication : public QMainWindow
{
    Q_OBJECT

public:
    QtWidgetsApplication(QWidget *parent = Q_NULLPTR);

    void show();

private:
    Ui::QtWidgetsApplicationClass ui;
    GUI::Viewer::Viewer viewer;
};
