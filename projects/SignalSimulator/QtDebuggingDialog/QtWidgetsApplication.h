#pragma once

#include <QtWidgets/QMainWindow>
#include "ui_QtWidgetsApplication.h"
#include "SimulatorListDialog_global.h"

#include "Utility.hpp"
#include "GeneralPurposeTimer.hpp"
#include "UnitOfNumber.hpp"
#include "ScientificPostulates.hpp"
#include "Simulator.hpp"

using Signal = Simulator::BaseSimulator;

class QtWidgetsApplication : public QMainWindow
{
    Q_OBJECT

private:
    class SignalInstanceUpdateFunction : public Simulator::BaseSimulator::ISignalInstanceUpdateFunction
    {
    public:
        static SignalInstanceUpdateFunction* GetInstance();
        void Registered(Simulator::BaseSimulator& rSignalInstance) override;
        void Delete(Simulator::BaseSimulator& rSignalInstance) override;
        void ValueUpdate(Simulator::BaseSimulator& rSignalInstance) override;
    };

public:
    QtWidgetsApplication(QWidget *parent = Q_NULLPTR);
    ~QtWidgetsApplication();

public slots:
    void ShowSimulatorListDialog();
    void CloseSimulatorListDialog();

protected:
    inline SimulatorListDialog* InstanceCreationForSimulatorListDialog();
    inline void InstanceDestroyedForSimulatorListDialog(SimulatorListDialog* p);

private:
    Ui::QtWidgetsApplicationClass ui;
    HMODULE hModuleSimulatorListDialog;
    static inline SimulatorListDialog* pSimulatorListDialog;
};
