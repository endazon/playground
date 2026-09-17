#pragma once

#include <mutex>
#include <QtWidgets/QWidget>
#include "GeneralPurposeTimer.hpp"
#include "ui_CommunicationHistoryListDialog.h"

class CommunicationHistoryListDialog : public QWidget
{
    Q_OBJECT

public:
    CommunicationHistoryListDialog(QWidget *parent = Q_NULLPTR);

    void AddMessage(std::string Type, std::string Msg);

private:
    Ui::CommunicationHistoryListDialogClass ui;
    static std::mutex _Mutex;
    GeneralPurposeTimer::DateFormat::UTC Time;
};
