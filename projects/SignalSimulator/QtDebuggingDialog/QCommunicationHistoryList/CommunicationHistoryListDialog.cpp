#include "CommunicationHistoryListDialog.h"

std::mutex CommunicationHistoryListDialog::_Mutex;
CommunicationHistoryListDialog::CommunicationHistoryListDialog(QWidget *parent)
: QWidget(parent)
{
    ui.setupUi(this);
}

void CommunicationHistoryListDialog::AddMessage(std::string Type, std::string Msg)
{
    std::lock_guard<std::mutex> lock(_Mutex);

    //準備
    auto table = ui.TableWidget;
    QTableWidgetItem* time = new QTableWidgetItem(QString::fromLocal8Bit(Time.format()));
    QTableWidgetItem* type = new QTableWidgetItem(QString::fromLocal8Bit(Type));
    QTableWidgetItem* msg  = new QTableWidgetItem(QString::fromLocal8Bit(Msg));
    const qsizetype rowCount = table->rowCount();
    if (rowCount < 0) { return; }

    //前処理
    table->setRowCount(rowCount + 1);

    //リスト処理
    {
        const bool sortingEnabled = table->isSortingEnabled();
        table->setSortingEnabled(false);
        table->setItem(rowCount, 0, time);
        table->setItem(rowCount, 1, type);
        table->setItem(rowCount, 2, msg);
        table->setCurrentIndex(table->currentIndex());
        table->setSortingEnabled(sortingEnabled);
    }

    //一番下にスクロール
    {
        auto item = table->item(rowCount, 0);
        table->scrollToItem(item, QAbstractItemView::PositionAtTop);
        table->selectRow(rowCount);
        table->selectionModel()->clear();
    }
}
