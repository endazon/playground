#pragma once

#include <mutex>
#include <thread>
#include <QtWidgets/QWidget>
#include "ui_SimulatorListDialog.h"

class DoublyLinkedList
{
protected:
    static DoublyLinkedList* head;
    static DoublyLinkedList* tail;

    DoublyLinkedList* prev;
    DoublyLinkedList* next;
    unsigned long long serialNumber;

public:
    DoublyLinkedList();

    void pushFront(DoublyLinkedList* list);
    void pushBack(DoublyLinkedList* list);
    DoublyLinkedList* pop();
    //void erase();
    //void insert();
    //void clear();
};

class SimulatorListDialog : public QWidget
{
    Q_OBJECT

private:
    class Element : public DoublyLinkedList
    {
    private:
        QTableWidgetItem SerialNumber;
        QTableWidgetItem Name;
        QTableWidgetItem Group;
        QTableWidgetItem Comment;
        QTableWidgetItem Value;

    public:
        Element(QString Name = "", QString Group = "", QString Comment = "", double Value = 0);
        Element(std::string Name = "", std::string Group = "", std::string Comment = "", double Value = 0);
        ~Element() noexcept;

        inline void setSerialNumber(int val) { SerialNumber.setText(QString::number(val)); }
        inline void setName(QString text)    { Name.setText(text);                         }
        inline void setGroup(QString text)   { Group.setText(text);                        }
        inline void setComment(QString text) { Comment.setText(text);                      }
        inline void setValue(double val)     { Value.setText(QString::number(val));        }
         
        void setupItem(QTableWidget& table);
        void removeItem(QTableWidget& table);
        void relocationItem(QTableWidget& table);
    };

    static std::mutex _Mutex;
    Ui::SimulatorListDialogClass ui;
    QHash<long long, Element*> List;

public:
    SimulatorListDialog(QWidget *parent = Q_NULLPTR);

    void AddElement(long long key, std::string Name, std::string Group, std::string Comment, long double Value);
    void RemovalElement(long long key);
    void ValueUpdate(long long key, long double Value);
};