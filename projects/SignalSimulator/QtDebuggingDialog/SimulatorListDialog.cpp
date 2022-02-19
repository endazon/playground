#include "SimulatorListDialog.h"

SimulatorListDialog::SimulatorListDialog(QWidget *parent)
    : QWidget(parent)
{
    ui.setupUi(this);

    ////‰ŠúÝ’è
    //{
    //    if (ui.TableWidget->rowCount() < 1)
    //        ui.TableWidget->setRowCount(1);
    //    QTableWidgetItem* __qtablewidgetitem = new QTableWidgetItem();
    //    ui.TableWidget->setVerticalHeaderItem(0, __qtablewidgetitem);
    //}
    //
    ////s
    //{
    //    QTableWidgetItem* ___qtablewidgetitem = ui.TableWidget->verticalHeaderItem(0);
    //    ___qtablewidgetitem->setText(QCoreApplication::translate("SimulatorListDialogClass", "1", nullptr));

    //    QTableWidgetItem* __qtablewidgetitem = new QTableWidgetItem();
    //    ui.TableWidget->setItem(0, 0, __qtablewidgetitem);
    //    QTableWidgetItem* __qtablewidgetitem1 = new QTableWidgetItem();
    //    ui.TableWidget->setItem(0, 1, __qtablewidgetitem1);
    //    QTableWidgetItem* __qtablewidgetitem2 = new QTableWidgetItem();
    //    ui.TableWidget->setItem(0, 2, __qtablewidgetitem2);
    //    QTableWidgetItem* __qtablewidgetitem3 = new QTableWidgetItem();
    //    ui.TableWidget->setItem(0, 3, __qtablewidgetitem3);
    //    QTableWidgetItem* __qtablewidgetitem4 = new QTableWidgetItem();
    //    ui.TableWidget->setItem(0, 4, __qtablewidgetitem4);
    //}
    //
    ////—v‘f
    //{
    //    const bool __sortingEnabled = ui.TableWidget->isSortingEnabled();
    //    ui.TableWidget->setSortingEnabled(false);
    //    QTableWidgetItem* ___qtablewidgetitem = ui.TableWidget->item(0, 0);
    //    ___qtablewidgetitem->setText(QCoreApplication::translate("SimulatorListDialogClass", "1", nullptr));
    //    QTableWidgetItem* ___qtablewidgetitem1 = ui.TableWidget->item(0, 1);
    //    ___qtablewidgetitem1->setText(QCoreApplication::translate("SimulatorListDialogClass", "2", nullptr));
    //    QTableWidgetItem* ___qtablewidgetitem2 = ui.TableWidget->item(0, 2);
    //    ___qtablewidgetitem2->setText(QCoreApplication::translate("SimulatorListDialogClass", "3", nullptr));
    //    QTableWidgetItem* ___qtablewidgetitem3 = ui.TableWidget->item(0, 3);
    //    ___qtablewidgetitem3->setText(QCoreApplication::translate("SimulatorListDialogClass", "4", nullptr));
    //    QTableWidgetItem* ___qtablewidgetitem4 = ui.TableWidget->item(0, 4);
    //    ___qtablewidgetitem4->setText(QCoreApplication::translate("SimulatorListDialogClass", "5", nullptr));
    //    ui.TableWidget->setSortingEnabled(__sortingEnabled);
    //}
}