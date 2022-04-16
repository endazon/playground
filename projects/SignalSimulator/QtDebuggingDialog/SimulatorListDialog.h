#pragma once

#include <mutex>
#include <thread>
#include <QtWidgets/QWidget>
#include "ui_SimulatorListDialog.h"

class SimulatorListDialog : public QWidget
{
    Q_OBJECT

public:
    struct Element
    {
        Element(QString Name = "", QString Group = "", QString Comment = "", long double Value = 0)
        : Name(Name), Group(Group), Comment(Comment), Value(Value){}

        QString     Name;
        QString     Group;
        QString     Comment;
        double      Value;

        QString toQString(int i)
        {
            switch (i)
            {
            case 0:  return Name;
            case 1:  return QString::number(Value);
            case 2:  return Comment;
            case 3:  return Group;
            default: return QString();
            }
        }
    };

    SimulatorListDialog(QWidget *parent = Q_NULLPTR);

    void AddElement(long long key, std::string Name, std::string Group, std::string Comment, long double Value);
    void RemovalElement(long long key);
    void ValueUpdate(long long key, long double Value);

private:
    Ui::SimulatorListDialogClass ui;
    inline static std::mutex _Mutex;

    class ElementList
    {
    private:
        using Key = long long;
        using T = Element;

        QList<Key> Keys;
        QHash<Key, T> Elements;

    public:
        ElementList() :Keys(), Elements() {}

        inline bool contains(Key key) const 
        {
            return Elements.contains(key);
        }

        inline qsizetype count() const
        {
            return Elements.count();
        }

        inline void append(Key key, T element)
        {
            if (contains(key)) { return; }
            Elements[key] = element;
            Keys.append(key);
        }

        inline void remove(Key key)
        {
            Elements.remove(key);
            Keys.remove(indexOf(key));
        }

        inline Element take(Key key)
        {
            if (!contains(key)) { return Element(); }
            Keys.remove(indexOf(key));
            return Elements.take(key);
        }

        inline qsizetype indexOf(Key key)
        {
            return Keys.indexOf(key);
        }

        inline T& operator[](Key key)&
        {
            return Elements[key];
        }
    }List;
};
