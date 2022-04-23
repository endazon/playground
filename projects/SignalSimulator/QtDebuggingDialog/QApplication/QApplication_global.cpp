#include <QtWidgets/QApplication>
#include "QApplication_global.h"

//QApplication
namespace {

// DLLインターフェースクラスの実装
class QApplicationForDLL : public IQApplication
{
private:
    using __MySelfType = QApplicationForDLL;

    QApplication ap;

    QApplicationForDLL(int argc, char* argv[])
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
SIMULATORLISTDIALOG_EXPORT QApplicationForDLL* InstanceCreation(int argc, char* argv[])
{
    return &QApplicationForDLL::GetInstance(argc, argv);
}

}