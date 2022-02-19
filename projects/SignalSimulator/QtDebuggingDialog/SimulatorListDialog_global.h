#pragma once

#include <QtCore/qglobal.h>
#include "SimulatorListDialog.h"

#if defined(SIMULATORLISTDIALOG_LIBRARY)
#  define SIMULATORLISTDIALOG_EXPORT Q_DECL_EXPORT
#else
#  define SIMULATORLISTDIALOG_EXPORT Q_DECL_IMPORT

typedef SimulatorListDialog*(*load_SimulatorListDialog_symbol)();
typedef void(*destroy_SimulatorListDialog_symbol)(SimulatorListDialog*);
#endif
