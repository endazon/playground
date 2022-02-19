# CMakeLists.txtを変更せずに設定を行う。
# 環境毎に柔軟に設定を行うことが可能

# 対象のCMake設定では/MDdと/MDが指定されるため、/MTdと/MTに直す
set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} /MTd /Zi /Ob0 /Od /RTC1" CACHE STRING "description")
set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} /MT /O2 /Ob2 /DNDEBUG" CACHE STRING "description")

# Qtインストール先
set(QTDIR "C:/03_liblary/Qt/6.2.3/msvc2019_64")

# Qtライブラリ群
set(QT_LIBRARY_DIR "C:/03_liblary/Bat/Qt/output")

message(STATUS "CMAKE_CXX_FLAGS_DEBUG = ${CMAKE_CXX_FLAGS_DEBUG}")
message(STATUS "CMAKE_CXX_FLAGS_RELEASE = ${CMAKE_CXX_FLAGS_RELEASE}")
message(STATUS "QTDIR = ${QTDIR}")
message(STATUS "QT_LIBRARY_DIR = ${QT_LIBRARY_DIR}")
