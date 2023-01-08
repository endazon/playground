#include "Viewer.h"

#include <QPainter>
#include <QTextBlock>

namespace GUI {
namespace Viewer {

QTextEditWithAddress::QTextEditWithAddress(bool headerValid, bool offsetAddressValid, QWidget* parent)
: QPlainTextEdit(parent)
, header(headerValid ? new Header(this) : new DummyHeader(this))
, offsetAddress(offsetAddressValid ? new OffsetAddress(this) : new DummyOffsetAddress(this))
, blank(headerValid && offsetAddressValid ? new Blank(this) : new DummyBlank(this))
{
    connect(this, &QTextEditWithAddress::updateRequest, this, &QTextEditWithAddress::updateLineNumber);
}

QSize QTextEditWithAddress::sizeHint() const
{
    return QSize(
        blank->sizeHint().width() + header->sizeHint().width(),
        0
    );
}

void QTextEditWithAddress::setHeaderText(QString text)
{
    header->setText(text);
}

void QTextEditWithAddress::updateLayout()
{
    setViewportMargins(
        header->geometry().left(),
        offsetAddress->geometry().top() - 1,
        0,
        0
    );
}

void QTextEditWithAddress::resizeEvent(QResizeEvent* event)
{
    __super::resizeEvent(event);

    QRect cr = contentsRect();
    blank->setGeometry(QRect(
        cr.left(),
        cr.top(),
        blank->sizeHint().width(),
        blank->sizeHint().height()
    ));
    header->setGeometry(QRect(
        blank->rect().left() + blank->sizeHint().width(),
        blank->rect().top(),
        cr.width(),
        header->sizeHint().height()
    ));
    offsetAddress->setGeometry(QRect(
        blank->rect().left(),
        blank->rect().top() + blank->sizeHint().height(),
        offsetAddress->sizeHint().width(),
        header->sizeHint().height()
    ));

    updateLayout();
}

void QTextEditWithAddress::updateLineNumber(const QRect& rect, int dy)
{
    if (dy)
    {
        offsetAddress->scroll(0, dy);
    }
    else
    {
        offsetAddress->update(0, rect.y(), offsetAddress->width(), rect.height());
    }

    if (rect.contains(viewport()->rect()))
    {
        updateLayout();
    }
}


Information::Information(QTextEditWithAddress* editor)
: QWidget(editor)
, textEditor(editor)
{}

QFontMetrics Information::textEditorFontMetrics() const
{
    return textEditor->fontMetrics();
}

Header::Header(QTextEditWithAddress* editor)
: Information(editor)
, Text("")
{}

void Header::setText(QString text)
{
    Text = text;
}

QSize Header::sizeHint() const
{
    return QSize(
        textEditorFontMetrics().horizontalAdvance(Text) + 4,
        textEditorFontMetrics().height()
    );
}

void Header::paintEvent(QPaintEvent* event)
{
    QPainter painter(this);
    painter.fillRect(event->rect(), QRgb(0xEEEEEE));

    painter.setPen(Qt::black);
    painter.drawText(4, 0, width(), height(),
        Qt::AlignLeft, Text);
}

OffsetAddress::OffsetAddress(QTextEditWithAddress* editor)
: Information(editor)
{}

QSize OffsetAddress::sizeHint() const
{
    return QSize(
        textEditorFontMetrics().horizontalAdvance("00:0000"),
        0
    );
}

void OffsetAddress::paintEvent(QPaintEvent* event)
{
    QPainter painter(this);
    painter.fillRect(event->rect(), QRgb(0xEEEEEE));

    QTextBlock block = textEditor->firstVisibleBlock();
    int blockNumber = block.blockNumber();
    int top = qRound(textEditor->blockBoundingGeometry(block).translated(textEditor->contentOffset()).top());
    int bottom = top + qRound(textEditor->blockBoundingRect(block).height());

    while (block.isValid() && top <= event->rect().bottom()) {
        if (block.isVisible() && bottom >= event->rect().top()) {
            auto numberStr = [](int blockNumber)
            {
                if (blockNumber & 0xFFFF0000)
                {
                    return QString::number((blockNumber >> 16) & 0x0000FFFF, 16).toUpper()
                         + ':'
                         + QString::number(blockNumber, 16).right(4).toUpper();
                }
                else
                {
                    return QString::number(blockNumber, 16).toUpper();
                }
            };
            QString number = numberStr(blockNumber * 0x10);
            painter.setPen(Qt::black);
            painter.drawText(0, top, width(), fontMetrics().height(),
                Qt::AlignRight, number);
            QString str = "";
            for (size_t i = QString("00:0000").size(); i; i--)
            {
                char chr = 0;
                switch (i)
                {
                case 5:  chr = ':'; break;
                default: chr = '0'; break;
                }
                if (i <= number.size()) { chr = ' '; }
                str += chr;
            }
            painter.setPen(Qt::white);
            painter.drawText(0, top, width(), fontMetrics().height(),
                Qt::AlignRight, str);
        }

        block = block.next();
        top = bottom;
        bottom = top + qRound(textEditor->blockBoundingRect(block).height());
        ++blockNumber;
    }
}

Blank::Blank(QTextEditWithAddress* editor)
: Information(editor)
, Text("")
{}

void Blank::setText(QString text)
{
    Text = text;
}

QSize Blank::sizeHint() const
{
    return QSize(
        textEditorFontMetrics().horizontalAdvance("00:0000") + 10,
        textEditorFontMetrics().height()
    );
}

void Blank::paintEvent(QPaintEvent* event)
{
    QPainter painter(this);
    painter.fillRect(event->rect(), QRgb(0xFFFFFF));

    painter.setPen(Qt::black);
    painter.drawText(4, 0, width(), height(),
        Qt::AlignLeft, Text);
}



MainEditor::MainEditor(QWidget* parent)
: QTextEditWithAddress(true, true, parent)
{}

void MainEditor::keyPressEvent(QKeyEvent* e)
{
    switch (e->key())
    {
    case Qt::Key_Return: return;
    case Qt::Key_Enter:  return;
    default:
        break;
    }
    __super::keyPressEvent(e);
}

SubEditor::SubEditor(QWidget* parent)
: QTextEditWithAddress(true, false, parent)
{}

Viewer::Viewer(QWidget* parent)
: QWidget(parent)
, mainEditor(parent)
, subEditor(parent)
{
    ui.setupUi(this);

    auto configuration = [](GUI::Viewer::QTextEditWithAddress& edit)
    {
        QFont font;
        font.setFamilies({ QString::fromUtf8("FixedSys") });
        font.setPointSize(14);
        edit.setFont(font);
        edit.setFrameShape(QFrame::NoFrame);
        edit.setFrameShadow(QFrame::Plain);
        edit.setSizePolicy(QSizePolicy::Minimum, QSizePolicy::Preferred);
        edit.setCursorWidth(edit.fontMetrics().horizontalAdvance(QLatin1Char('9')));
    };

    configuration(mainEditor);
    mainEditor.setHeaderText(QCoreApplication::translate("ViewerClass", "+0 +1 +2 +3 +4 +5 +6 +7 +8 +9 +A +B +C +D +E +F ", nullptr));
    ui.gridLayout->addWidget(&mainEditor, 0, 0, 1, 1);

    configuration(subEditor);
    subEditor.setHeaderText(QCoreApplication::translate("ViewerClass", "0123456789ABCDEF ", nullptr));
    ui.gridLayout->addWidget(&subEditor, 0, 1, 1, 1);

    setFixedWidth(sizeHint().width());
}

QSize Viewer::sizeHint() const
{
    return QSize(
        mainEditor.sizeHint().width() + subEditor.sizeHint().width(),
        0
    );
}

}
}


///********************************************************************************
//** Form generated from reading UI file 'Viewer.ui'
//**
//** Created by: Qt User Interface Compiler version 6.2.3
//**
//** WARNING! All changes made in this file will be lost when recompiling UI file!
//********************************************************************************/
//
//#ifndef UI_VIEWER_H
//#define UI_VIEWER_H
//
//#include <QtCore/QVariant>
//#include <QtWidgets/QApplication>
//#include <QtWidgets/QGridLayout>
//#include <QtWidgets/QLabel>
//#include <QtWidgets/QTextEdit>
//#include <QtWidgets/QWidget>
//
//QT_BEGIN_NAMESPACE
//
//class Ui_ViewerClass
//{
//public:
//    QGridLayout* gridLayout;
//    QLabel* label_2;
//    QLabel* label;
//    QTextEdit* textEdit;
//    QTextEdit* textEdit_3;
//    QTextEdit* textEdit_2;
//    QLabel* label_3;
//
//    void setupUi(QWidget* ViewerClass)
//    {
//        if (ViewerClass->objectName().isEmpty())
//            ViewerClass->setObjectName(QString::fromUtf8("ViewerClass"));
//        ViewerClass->resize(581, 482);
//        ViewerClass->setStyleSheet(QString::fromUtf8(""));
//        gridLayout = new QGridLayout(ViewerClass);
//        gridLayout->setSpacing(0);
//        gridLayout->setContentsMargins(11, 11, 11, 11);
//        gridLayout->setObjectName(QString::fromUtf8("gridLayout"));
//        gridLayout->setContentsMargins(0, 0, 0, 0);
//        label_2 = new QLabel(ViewerClass);
//        label_2->setObjectName(QString::fromUtf8("label_2"));
//        QFont font;
//        font.setFamilies({ QString::fromUtf8("FixedSys") });
//        font.setPointSize(14);
//        label_2->setFont(font);
//        label_2->setStyleSheet(QString::fromUtf8("background-color : #eeeeee"));
//        label_2->setFrameShape(QFrame::NoFrame);
//        label_2->setLineWidth(0);
//
//        gridLayout->addWidget(label_2, 0, 1, 1, 1);
//
//        label = new QLabel(ViewerClass);
//        label->setObjectName(QString::fromUtf8("label"));
//        label->setFont(font);
//        label->setStyleSheet(QString::fromUtf8("background-color : #eeeeee"));
//        label->setLineWidth(0);
//        label->setAlignment(Qt::AlignCenter);
//
//        gridLayout->addWidget(label, 0, 0, 1, 1);
//
//        textEdit = new QTextEdit(ViewerClass);
//        textEdit->setObjectName(QString::fromUtf8("textEdit"));
//        textEdit->setFont(font);
//        textEdit->setStyleSheet(QString::fromUtf8(""));
//        textEdit->setFrameShape(QFrame::NoFrame);
//        textEdit->setFrameShadow(QFrame::Plain);
//        textEdit->setLineWidth(0);
//        textEdit->setReadOnly(true);
//        textEdit->setAcceptRichText(true);
//
//        gridLayout->addWidget(textEdit, 1, 0, 1, 1);
//
//        textEdit_3 = new QTextEdit(ViewerClass);
//        textEdit_3->setObjectName(QString::fromUtf8("textEdit_3"));
//        textEdit_3->setFont(font);
//        textEdit_3->setFrameShape(QFrame::NoFrame);
//        textEdit_3->setFrameShadow(QFrame::Plain);
//        textEdit_3->setLineWidth(0);
//
//        gridLayout->addWidget(textEdit_3, 1, 2, 1, 1);
//
//        textEdit_2 = new QTextEdit(ViewerClass);
//        textEdit_2->setObjectName(QString::fromUtf8("textEdit_2"));
//        textEdit_2->setFont(font);
//        textEdit_2->setFrameShape(QFrame::NoFrame);
//        textEdit_2->setFrameShadow(QFrame::Plain);
//        textEdit_2->setLineWidth(0);
//        textEdit_2->setLineWrapMode(QTextEdit::WidgetWidth);
//        textEdit_2->setLineWrapColumnOrWidth(0);
//
//        gridLayout->addWidget(textEdit_2, 1, 1, 1, 1);
//
//        label_3 = new QLabel(ViewerClass);
//        label_3->setObjectName(QString::fromUtf8("label_3"));
//        label_3->setFont(font);
//        label_3->setStyleSheet(QString::fromUtf8("background-color : #eeeeee"));
//        label_3->setLineWidth(0);
//
//        gridLayout->addWidget(label_3, 0, 2, 1, 1);
//
//
//        retranslateUi(ViewerClass);
//
//        QMetaObject::connectSlotsByName(ViewerClass);
//    } // setupUi
//
//    void retranslateUi(QWidget* ViewerClass)
//    {
//        ViewerClass->setWindowTitle(QCoreApplication::translate("ViewerClass", "Viewer", nullptr));
//        ViewerClass->setWindowFilePath(QString());
//        label_2->setText(QCoreApplication::translate("ViewerClass", "+0 +1 +2 +3 +4 +5 +6 +7 +8 +9 +A +B +C +D +E +F ", nullptr));
//        label->setText(QCoreApplication::translate("ViewerClass", "Address", nullptr));
//        textEdit->setHtml(QCoreApplication::translate("ViewerClass", "<!DOCTYPE HTML PUBLIC \"-//W3C//DTD HTML 4.0//EN\" \"http://www.w3.org/TR/REC-html40/strict.dtd\">\n"
//            "<html><head><meta name=\"qrichtext\" content=\"1\" /><meta charset=\"utf-8\" /><style type=\"text/css\">\n"
//            "p, li { white-space: pre-wrap; }\n"
//            "</style></head><body style=\" font-family:'FixedSys'; font-size:14pt; font-weight:400; font-style:normal;\">\n"
//            "<p style=\" margin-top:0px; margin-bottom:0px; margin-left:0px; margin-right:0px; -qt-block-indent:0; text-indent:0px;\"><span style=\" color:#ffffff; background-color:#eeeeee;\">00:000</span><span style=\" color:#000000; background-color:#eeeeee;\">1</span></p></body></html>", nullptr));
//        textEdit_3->setHtml(QCoreApplication::translate("ViewerClass", "<!DOCTYPE HTML PUBLIC \"-//W3C//DTD HTML 4.0//EN\" \"http://www.w3.org/TR/REC-html40/strict.dtd\">\n"
//            "<html><head><meta name=\"qrichtext\" content=\"1\" /><meta charset=\"utf-8\" /><style type=\"text/css\">\n"
//            "p, li { white-space: pre-wrap; }\n"
//            "</style></head><body style=\" font-family:'FixedSys'; font-size:14pt; font-weight:400; font-style:normal;\">\n"
//            "<p style=\" margin-top:0px; margin-bottom:0px; margin-left:0px; margin-right:0px; -qt-block-indent:0; text-indent:0px;\">ABCDEF000000000000000000000000</p></body></html>", nullptr));
//        textEdit_2->setHtml(QCoreApplication::translate("ViewerClass", "<!DOCTYPE HTML PUBLIC \"-//W3C//DTD HTML 4.0//EN\" \"http://www.w3.org/TR/REC-html40/strict.dtd\">\n"
//            "<html><head><meta name=\"qrichtext\" content=\"1\" /><meta charset=\"utf-8\" /><style type=\"text/css\">\n"
//            "p, li { white-space: pre-wrap; }\n"
//            "</style></head><body style=\" font-family:'FixedSys'; font-size:14pt; font-weight:400; font-style:normal;\">\n"
//            "<p style=\" margin-top:0px; margin-bottom:0px; margin-left:0px; margin-right:0px; -qt-block-indent:0; text-indent:0px;\">123456AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA</p></body></html>", nullptr));
//        label_3->setText(QCoreApplication::translate("ViewerClass", "0123456789ABCDEF", nullptr));
//    } // retranslateUi
//
//};
//
//namespace Ui {
//    class ViewerClass : public Ui_ViewerClass {};
//} // namespace Ui
//
//QT_END_NAMESPACE
//
//#endif // UI_VIEWER_H
