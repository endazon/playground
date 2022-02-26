#include "SimulatorListDialog.h"

SimulatorListDialog::SimulatorListDialog(QWidget *parent)
: QWidget(parent)
, List()
{
    ui.setupUi(this);
}

void SimulatorListDialog::AddElement(long long key, std::string Name, std::string Group, std::string Comment, long double Value)
{
    std::lock_guard<std::mutex> lock(_Mutex);

    //初期設定
    if (List.contains(key)) { return; }
    //Element element = Element(QString::fromStdString(Name), QString::fromStdString(Group), QString::fromStdString(Comment), Value);
    //Element element = Element(QString::fromUtf8(Name), QString::fromUtf8(Group), QString::fromUtf8(Comment), Value);
    Element element = Element(QString::fromLocal8Bit(Name), QString::fromLocal8Bit(Group), QString::fromLocal8Bit(Comment), Value);
    List.append(key, element);
    const int numberOfLists = List.count();
    const int addOffset = numberOfLists - 1;
    if (numberOfLists < 1) { return; }
    ui.TableWidget->setRowCount(numberOfLists);

    //行ヘッダ追加
    QTableWidgetItem* qtablewidgetverticalheaderitem = new QTableWidgetItem();
    qtablewidgetverticalheaderitem->setText(QString::number(List.indexOf(key) + 1));
    ui.TableWidget->setVerticalHeaderItem(addOffset, qtablewidgetverticalheaderitem);

    //行要素追加
    const bool sortingEnabled = ui.TableWidget->isSortingEnabled();
    ui.TableWidget->setSortingEnabled(false);
    for (int i = 0; i < ui.TableWidget->columnCount(); i++)
    {
        QTableWidgetItem* qtablewidgetitem = new QTableWidgetItem();
        qtablewidgetitem->setText(element.toQString(i));
        ui.TableWidget->setItem(addOffset, i, qtablewidgetitem);
    }
    ui.TableWidget->setSortingEnabled(sortingEnabled);
}

void SimulatorListDialog::RemovalElement(long long key)
{
    std::lock_guard<std::mutex> lock(_Mutex);

    //初期設定
    if (!List.contains(key)) { return; }
    qsizetype no = List.indexOf(key);
    List.remove(key);
    const int numberOfLists = List.count();
    const int addOffset = numberOfLists - 1;
    if (numberOfLists < 1) { return; }

    //行削除
    delete ui.TableWidget->takeVerticalHeaderItem(no);
    for (int i = 0; i < ui.TableWidget->columnCount(); i++)
    {
        delete ui.TableWidget->takeItem(no, i);
    }

    for (qsizetype i = no + 1; i < numberOfLists + 1; i++)
    {
        //行ヘッダ再配置
        QTableWidgetItem* qtablewidgetverticalheaderitem = ui.TableWidget->takeVerticalHeaderItem(i);
        qtablewidgetverticalheaderitem->setText(QString::number(i));
        ui.TableWidget->setVerticalHeaderItem(i - 1, qtablewidgetverticalheaderitem);

        //行要素再配置
        const bool sortingEnabled = ui.TableWidget->isSortingEnabled();
        ui.TableWidget->setSortingEnabled(false);
        for (int j = 0; j < ui.TableWidget->columnCount(); j++)
        {
            QTableWidgetItem* qtablewidgetitem = ui.TableWidget->takeItem(i, j);
            ui.TableWidget->setItem(i - 1, j, qtablewidgetitem);
        }
        ui.TableWidget->setSortingEnabled(sortingEnabled);
    }

    ui.TableWidget->setRowCount(numberOfLists);
}

void SimulatorListDialog::ValueUpdate(long long key, long double Value)
{
    std::lock_guard<std::mutex> lock(_Mutex);
    if (!List.contains(key)) { return; }
    Element& element = List[key];
    element.Value = Value;

    QTableWidgetItem* qtablewidgetitem = ui.TableWidget->item(List.indexOf(key), 3);
    qtablewidgetitem->setText(element.toQString(3));
}