# CMakeList.txt : Training の CMake プロジェクト。ソースを含めて、次を定義します:
# プロジェクト専用ロジックはこちらです。
#
cmake_minimum_required(VERSION 3.8)

# サブディレクトリを登録
add_subdirectory(original/Bz)

# ソースをこのプロジェクトの実行可能ファイルに追加します。