#include <QtWidgets/QApplication>
#include "QApplication_global.h"

//QApplication
namespace{

// DLLインターフェースクラスの実装
class QApplicationOFDLL : public IQApplication
{
private:
    using __MySelfType = QApplicationOFDLL;

    QApplication ap;

    QApplicationOFDLL(int argc, char* argv[])
    : ap(argc, argv)
    {}


public:
    inline static __MySelfType& GetInstance(int argc, char* argv[])
    {
        static __MySelfType instance(argc, argv);
        return instance;
    }

    //Qt
    int exec() override
    {
        return ap.exec();
    }
};

// エクスポート関数の実装
SIMULATORLISTDIALOG_EXPORT QApplicationOFDLL* load_QApplication_symbol(int argc, char* argv[])
{
    return &QApplicationOFDLL::GetInstance(argc, argv);
}

}