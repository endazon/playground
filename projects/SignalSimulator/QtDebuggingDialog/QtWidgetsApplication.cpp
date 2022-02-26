#include <windows.h>
#include "QtWidgetsApplication.h"

class TimeSimulateTest : public Simulator::BaseTimeSimulator
{
private:
    using __MySelfType = TimeSimulateTest;
    using __InheritanceType = BaseTimeSimulator;

protected:
    inline void UpdateIn100usCycle() noexcept override {}
    inline void UpdateIn200usCycle() noexcept override {}
    inline void UpdateIn500usCycle() noexcept override {}
    inline void UpdateIn1msCycle()   noexcept override {}
    inline void UpdateIn2msCycle()   noexcept override {}
    inline void UpdateIn5msCycle()   noexcept override {}
    inline void UpdateIn10msCycle()  noexcept override {}
    inline void UpdateIn20msCycle()  noexcept override {}
    inline void UpdateIn50msCycle()  noexcept override {}
    inline void UpdateIn100msCycle() noexcept override {}
    inline void UpdateIn200msCycle() noexcept override {}
    inline void UpdateIn500msCycle() noexcept override {}
    inline void UpdateIn1000msCycle()noexcept override
    {
        static Signal timer(0, "Timer", "TimeSimulateTest", "タイマ");
        timer++;

        //new Signal(timer * 2, "temporary", "TimeSimulateTest");
        if (temporary == nullptr) {
            temporary = new Signal(timer * 2, "temporary", "TimeSimulateTest");
        }
        else {
            delete temporary;
            temporary = nullptr;
        }
    }
    static inline Signal* temporary = nullptr;

public:
    using __InheritanceType::__InheritanceType; //継承元のコンストラクタは使わない

    //代入演算子(Assignment)
    //**********************************************************
    //暗黙的に宣言される
    //__MySelfType& operator=(const __MySelfType&) noexcept = delete;
    //__MySelfType& operator=(__MySelfType&&) & noexcept = delete;
    //**********************************************************
    template<class T> inline __MySelfType& operator=(T& rhs) noexcept
    {
        __InheritanceType::operator=(rhs);
        return *this;
    }

    template<class T> inline __MySelfType& operator=(T&& rhs) & noexcept
    {
        return operator=(rhs);
    }
}__TimeSimulateTest(0, "TimeSimulateTest", "TimeSimulateTest", "タイムシミュレーター");

QtWidgetsApplication::SignalInstanceUpdateFunction* QtWidgetsApplication::SignalInstanceUpdateFunction::GetInstance()
{
    static SignalInstanceUpdateFunction instance; 
    return &instance; 
}
void QtWidgetsApplication::SignalInstanceUpdateFunction::Registered(Simulator::BaseSimulator& rSignalInstance)
{
    QtWidgetsApplication::pSimulatorListDialog->AddElement(
        reinterpret_cast<long long>(&rSignalInstance),
        std::string(rSignalInstance.Name().data(), rSignalInstance.Name().size()),
        std::string(rSignalInstance.Group().data(), rSignalInstance.Group().size()),
        std::string(rSignalInstance.Comment().data(), rSignalInstance.Comment().size()),
        static_cast<Simulator::BaseSimulator::__ValueType>(rSignalInstance)
    );
}

void QtWidgetsApplication::SignalInstanceUpdateFunction::Delete(Simulator::BaseSimulator& rSignalInstance)
{
    QtWidgetsApplication::pSimulatorListDialog->RemovalElement(reinterpret_cast<long long>(&rSignalInstance));
}

void QtWidgetsApplication::SignalInstanceUpdateFunction::ValueUpdate(Simulator::BaseSimulator& rSignalInstance)
{
    QtWidgetsApplication::pSimulatorListDialog->ValueUpdate(
        reinterpret_cast<long long>(&rSignalInstance),
        static_cast<Simulator::BaseSimulator::__ValueType>(rSignalInstance)
    );
}

QtWidgetsApplication::QtWidgetsApplication(QWidget *parent)
: QMainWindow(parent)
{
    ui.setupUi(this);
    pSimulatorListDialog = new SimulatorListDialog();       //静的リンク時
    //pSimulatorListDialog = new SimulatorListDialogOfDLL();  //動的リンク時
    Simulator::BaseSimulator::RegisterSignalListAcquisitionFunction(SignalInstanceUpdateFunction::GetInstance());

    static Signal _Signal[10]=
    {
        {  1, "Test 1", "テスト", "☆★☆彡" },
        {  2, "Test 2", "テスト", "☆★☆彡" },
        {  3, "Test 3", "テスト", "☆★☆彡" },
        {  4, "Test 4", "テスト", "☆★☆彡" },
        {  5, "Test 5", "テスト", "☆★☆彡" },
        {  6, "Test 6", "テスト", "☆★☆彡" },
        {  7, "Test 7", "テスト", "☆★☆彡" },
        {  8, "Test 8", "テスト", "☆★☆彡" },
        {  9, "Test 9", "テスト", "☆★☆彡" },
        { 10, "Test10", "テスト", "☆★☆彡" },
    };
}

QtWidgetsApplication::~QtWidgetsApplication()
{
    Simulator::BaseSimulator::DeleteSignalListAcquisitionFunction();
    delete pSimulatorListDialog;
}

void QtWidgetsApplication::ShowSimulatorListDialog()
{
    if (pSimulatorListDialog->isVisible()) { return; }
    pSimulatorListDialog->show();
    //pSimulatorListDialog->showMaximized();
}

void QtWidgetsApplication::CloseSimulatorListDialog()
{
    if (pSimulatorListDialog->isHidden()) { return; }
    pSimulatorListDialog->close();
}