USE_QT ?= 0

ifeq ($(USE_QT),1)
  # Ici: tes vrais wrappers Qt + pkg-config/qt* pour flags & libs
  QT_SRC = desktop/qt_real.cpp
  QT_LIBS = $(shell pkg-config --libs Qt5Widgets 2>/dev/null || echo "-lQt5Widgets")
  QT_CXXFLAGS = $(shell pkg-config --cflags Qt5Widgets 2>/dev/null)
else
  QT_SRC = desktop/qt_stub.cpp
  QT_LIBS =
  QT_CXXFLAGS =
endif

build:
	# 1) Backend (stub ou réel)
	c++ -std=c++17 -O2 -c $(QT_CXXFLAGS) $(QT_SRC) -o build/qt_backend.o
	# 2) Compile Vitte → objet
	vittec build desktop/main.vitte -o build/app.o
	# 3) Link final
	c++ build/app.o build/qt_backend.o $(QT_LIBS) -o bin/vitte-desktop
