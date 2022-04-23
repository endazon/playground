#include "SimulatorListDialog.h"

DoublyLinkedList* DoublyLinkedList::head = nullptr;
DoublyLinkedList* DoublyLinkedList::tail = nullptr;
DoublyLinkedList::DoublyLinkedList()
: prev(nullptr)
, next(nullptr)
 ,serialNumber(0)
{
    if (head == nullptr) { head = this; }
    if (tail == nullptr) { tail = this; }
}

void DoublyLinkedList::pushFront(DoublyLinkedList* list)
{
    qulonglong number = this->serialNumber;

    if (head == this)
    {
        list->next = this;
        this->prev = list;

        head = list;
    }
    else 
    {
        list->next = this;
        list->prev = this->prev;
        this->prev = list;
    }

    for (DoublyLinkedList* current = list; current != nullptr; current = current->next)
    {
        current->serialNumber = number;
        number++;
    }
}

void DoublyLinkedList::pushBack(DoublyLinkedList* list)
{
    qulonglong number = this->serialNumber + 1;

    if (tail == this)
    {
        this->next = list;
        list->prev = this;

        tail = list;
    }
    else
    {
        list->next = this->next;
        list->prev = this;
        this->next = list;
    }

    for (DoublyLinkedList* current = list; current != nullptr; current = current->next)
    {
        current->serialNumber = number;
        number++;
    }
}

DoublyLinkedList* DoublyLinkedList::pop()
{
    qulonglong number = serialNumber;
    DoublyLinkedList* front = prev;
    DoublyLinkedList* back  = next;
    
    if (front != nullptr) { if (front->next == back) { return nullptr; } front->next = back; }
    if (back != nullptr)  { if (front == back->prev) { return nullptr; } back->prev = front; }
    if (head == this) { head = back;  }
    if (tail == this) { tail = front; }

    for (DoublyLinkedList* current = back; current != nullptr; current = current->next)
    {
        current->serialNumber = number;
        number++;
    }

    return this;
}

SimulatorListDialog::Element::Element(QString Name, QString Group, QString Comment, double Value)
: DoublyLinkedList()
, SerialNumber("")
, Name(Name)
, Group(Group)
, Comment(Comment)
, Value(QString::number(Value))
{
    if (tail != this)
    {
        tail->pushBack(this);
    }
}
SimulatorListDialog::Element::Element(std::string Name, std::string Group, std::string Comment, double Value)
: SimulatorListDialog::Element::Element(QString::fromLocal8Bit(Name), QString::fromLocal8Bit(Group), QString::fromLocal8Bit(Comment), Value)
{}

SimulatorListDialog::Element::~Element() noexcept
{
    pop();
}

void SimulatorListDialog::Element::setupItem(QTableWidget& table)
{
    const int row = serialNumber;
    int column = 0;

    //初期設定
    setSerialNumber(row + 1);

    //行ヘッダ追加
    table.setVerticalHeaderItem(row, &SerialNumber);

    //行要素追加
    table.setItem(row, column++, &Name);
    table.setItem(row, column++, &Group);
    table.setItem(row, column++, &Comment);
    table.setItem(row, column++, &Value);
}

void SimulatorListDialog::Element::removeItem(QTableWidget& table)
{
    const int row = serialNumber;
    int column = 0;

    //初期設定

    //行削除
    table.takeVerticalHeaderItem(row);

    //行要素削除
    table.takeItem(row, column++);
    table.takeItem(row, column++);
    table.takeItem(row, column++);
    table.takeItem(row, column++);
}

void SimulatorListDialog::Element::relocationItem(QTableWidget& table)
{
    for (Element* current = reinterpret_cast<Element*>(prev); current != nullptr; current = reinterpret_cast<Element*>(current->next))
    {
        current->removeItem(table);
        current->setupItem(table);
    }
}

std::mutex SimulatorListDialog::_Mutex;
SimulatorListDialog::SimulatorListDialog(QWidget *parent)
: QWidget(parent)
, List()
{
    ui.setupUi(this);
}

void SimulatorListDialog::AddElement(long long key, std::string Name, std::string Group, std::string Comment, long double Value)
{
    std::lock_guard<std::mutex> lock(_Mutex);

    if (List.contains(key)) { return; }
    auto table = ui.TableWidget;
    auto element = new SimulatorListDialog::Element(Name, Group, Comment, Value);
    List[key] = element;
    const qsizetype rowCount = List.size();
    if (rowCount < 1) { return; }

    table->setRowCount(rowCount);
    const bool sortingEnabled = table->isSortingEnabled();
    table->setSortingEnabled(false);
    element->setupItem(*table);
    table->setSortingEnabled(sortingEnabled);
}

void SimulatorListDialog::RemovalElement(long long key)
{
    std::lock_guard<std::mutex> lock(_Mutex);

    if (!List.contains(key)) { return; }
    auto table = ui.TableWidget;
    auto element = static_cast<SimulatorListDialog::Element*>(List.take(key)->pop());
    const qsizetype rowCount = List.size();
    if (rowCount < 1) { return; }

    const bool sortingEnabled = table->isSortingEnabled();
    table->setSortingEnabled(false);
    element->relocationItem(*table);
    table->setSortingEnabled(sortingEnabled);
    delete element;
    table->setRowCount(rowCount);
}

void SimulatorListDialog::ValueUpdate(long long key, long double Value)
{
    std::lock_guard<std::mutex> lock(_Mutex);

    if (!List.contains(key)) { return; }
    auto element = List[key];
    element->setValue(Value);
}