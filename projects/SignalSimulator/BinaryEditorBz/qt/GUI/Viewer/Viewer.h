#pragma once

#include <QtWidgets/QWidget>
#include <QPlainTextEdit>
#include "ui_Viewer.h"

class QPaintEvent;
class QResizeEvent;
class QSize;
class QWidget;

namespace GUI {
namespace Viewer {
        
#pragma region BaseQTextEditWithAddress

class Header;
class OffsetAddress;
class Blank;

class QTextEditWithAddress : public QPlainTextEdit
{
    Q_OBJECT
        
    friend Header;
    friend OffsetAddress;
    friend Blank;

private:
    Header* header;
    OffsetAddress* offsetAddress;
    Blank* blank;

private slots:
    void updateLineNumber(const QRect& rect, int dy);

protected:
    void updateLayout();
    void resizeEvent(QResizeEvent* event) override;

public:
    QTextEditWithAddress(bool headerValid, bool offsetAddressValid, QWidget* parent = nullptr);
    QSize sizeHint() const override;
    void setHeaderText(QString text);
};

class Information : public QWidget
{
protected:
    QTextEditWithAddress* textEditor;

public:
    Information(QTextEditWithAddress* editor);
    inline QFontMetrics textEditorFontMetrics() const;
};

class Header : public Information
{
private:
    QString Text;

protected:
    void paintEvent(QPaintEvent* event) override;

public:
    Header(QTextEditWithAddress* editor);

    void setText(QString text);
    QSize sizeHint() const override;
};

class DummyHeader : public Header
{
protected:
    void paintEvent(QPaintEvent* event) override{}

public:
    DummyHeader(QTextEditWithAddress* editor) : Header(editor) {}

    void setText(QString text){}
    QSize sizeHint() const override { return QSize(0, 0); }
};

class OffsetAddress : public Information
{
protected:
    void paintEvent(QPaintEvent* event) override;

public:
    OffsetAddress(QTextEditWithAddress* editor);

    QSize sizeHint() const override;
};

class DummyOffsetAddress : public OffsetAddress
{
protected:
    void paintEvent(QPaintEvent* event) override{}

public:
    DummyOffsetAddress(QTextEditWithAddress* editor) : OffsetAddress(editor) {}

    QSize sizeHint() const override { return QSize(0, 0); }
};

class Blank : public Information
{
private:
    QString Text;

protected:
    void paintEvent(QPaintEvent* event) override;

public:
    Blank(QTextEditWithAddress* editor);

    void setText(QString text);
    QSize sizeHint() const override;
};

class DummyBlank : public Blank
{
protected:
    void paintEvent(QPaintEvent* event) override {}

public:
    DummyBlank(QTextEditWithAddress* editor) : Blank(editor) {}

    void setText(QString text) {}
    QSize sizeHint() const override { return QSize(0, 0); }
};

#pragma endregion

class MainEditor : public QTextEditWithAddress
{
    Q_OBJECT

public:
    MainEditor(QWidget* parent = nullptr);

    void keyPressEvent(QKeyEvent* e) override;
};

class SubEditor : public QTextEditWithAddress
{
    Q_OBJECT

public:
    SubEditor(QWidget* parent = nullptr);
};

class Viewer : public QWidget
{
    Q_OBJECT

public:
    Viewer(QWidget* parent = nullptr);
    QSize sizeHint() const override;

private:
    GUI::Viewer::MainEditor mainEditor;
    GUI::Viewer::SubEditor subEditor;
    Ui::ViewerClass ui;
};

}
}