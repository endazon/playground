# CMakeList.txt : Training の CMake プロジェクト。ソースを含めて、次を定義します:
# プロジェクト専用ロジックはこちらです。
#
cmake_minimum_required(VERSION 3.8)

# ソースをこのプロジェクトの実行可能ファイルに追加します。
add_executable(DebuggingConsole DebuggingConsole.cpp DebuggingConsole.h)
target_compile_features(DebuggingConsole PUBLIC cxx_std_20)