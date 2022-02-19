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

add_executable(QtDebuggingDialog QtDebuggingDialog.cpp QtWidgetsApplication.cpp QtWidgetsApplication.ui QtWidgetsApplication.qrc)
target_link_libraries(QtDebuggingDialog Qt6::Core Qt6::Gui Qt6::Widgets)
target_compile_features(QtDebuggingDialog PUBLIC cxx_std_20)

# ビルド後処理
add_custom_command(
  TARGET QtDebuggingDialog 
  #PRE_BUILD	# ビルド前
  #PRE_LINK		# リンク前
  POST_BUILD	# ビルド後
  COMMAND ${CMAKE_COMMAND} -E copy_directory ${QT_LIBRARY_DIR} $<TARGET_FILE_DIR:QtDebuggingDialog>
)
