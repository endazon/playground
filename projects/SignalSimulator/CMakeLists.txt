# CMakeList.txt : Training の CMake プロジェクト。ソースを含めて、次を定義します:
# プロジェクト専用ロジックはこちらです。
#
cmake_minimum_required(VERSION 3.8)

project(Training)

enable_testing()

include_directories (
        ${PROJECT_SOURCE_DIR}/Target/include
)

# サブディレクトリを登録
add_subdirectory(Target)
add_subdirectory(DebuggingConsole)
add_subdirectory(QtDebuggingDialog)
#add_subdirectory(UnitTest)
