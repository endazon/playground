# CMakeList.txt : Training の CMake プロジェクト。ソースを含めて、次を定義します:
# プロジェクト専用ロジックはこちらです。
#
cmake_minimum_required(VERSION 3.8)

# ソースをこのプロジェクトの実行可能ファイルに追加します。
set(CMAKE_PREFIX_PATH ${QTDIR})

set(CMAKE_INCLUDE_CURRENT_DIR ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

find_package(Qt6 COMPONENTS Core Gui Widgets REQUIRED)

# CommunicationHistoryListDialog
if (QT_BUILD_SHARED_LIBS)
	add_library(CommunicationHistoryListDialog SHARED CommunicationHistoryListDialog.cpp CommunicationHistoryListDialog.ui CommunicationHistoryListDialog.qrc CommunicationHistoryListDialog_global.cpp)
else()
	add_library(CommunicationHistoryListDialog STATIC CommunicationHistoryListDialog.cpp CommunicationHistoryListDialog.ui CommunicationHistoryListDialog.qrc CommunicationHistoryListDialog_global.cpp)
endif()
target_link_libraries(CommunicationHistoryListDialog Qt6::Core Qt6::Gui Qt6::Widgets)
target_compile_features(CommunicationHistoryListDialog PUBLIC cxx_std_20)
target_compile_definitions(CommunicationHistoryListDialog PRIVATE COMMUNICATIONHISTORYLISTDIALOG_LIBRARY)	
