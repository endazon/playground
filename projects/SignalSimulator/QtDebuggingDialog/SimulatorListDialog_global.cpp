#include "SimulatorListDialog_global.h"

extern "C" SIMULATORLISTDIALOG_EXPORT SimulatorListDialog * load_SimulatorListDialog_symbol()
{
    return new SimulatorListDialog;
}

extern "C" SIMULATORLISTDIALOG_EXPORT void destroy_SimulatorListDialog_symbol(SimulatorListDialog * p)
{
    delete p;
}