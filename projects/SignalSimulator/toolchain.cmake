# CMakeLists.txtを変更せずに設定を行う。
# 環境毎に柔軟に設定を行うことが可能

# GoogleTestはMTでコンパイルされているため、こちらのプログラムもMTでコンパイルする必要がある
#set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} /MTd /Zi /Ob0 /Od /RTC1" CACHE STRING "description")
#set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} /MT /O2 /Ob2 /DNDEBUG" CACHE STRING "description")

# DLLを使用する場合がMDでコンパイルする必要がある。以下参照
# https://stackoverflow.com/questions/35310117/debug-assertion-failed-expression-acrt-first-block-header
set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} /MDd /Zi /Ob0 /Od /RTC1" CACHE STRING "description")
set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} /MD /O2 /Ob2 /DNDEBUG" CACHE STRING "description")

# Qtインストール先
set(QTDIR "C:/03_liblary/Qt/6.2.3/msvc2019_64")

# Qtライブラリ群
set(QT_LIBRARY_DIR "C:/03_liblary/Bat/Qt/output")

message(STATUS "CMAKE_CXX_FLAGS_DEBUG = ${CMAKE_CXX_FLAGS_DEBUG}")
message(STATUS "CMAKE_CXX_FLAGS_RELEASE = ${CMAKE_CXX_FLAGS_RELEASE}")
message(STATUS "QTDIR = ${QTDIR}")
message(STATUS "QT_LIBRARY_DIR = ${QT_LIBRARY_DIR}")
