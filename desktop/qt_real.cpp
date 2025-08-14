// desktop/qt_real.cpp
// Backend Qt "réel" — implémente des widgets effectifs via Qt Widgets.
// API C exposée (ABI stable pour FFI):
//   void   qt_init(int *argc, char ***argv);
//   void*  qt_window_new(const char* title, int w, int h);
//   void*  qt_button_new(const char* label);
//   void   qt_widget_set_parent(void* child, void* parent);
//   void   qt_widget_show(void* widget);
//   void   qt_widget_set_title(void* widget, const char* title);
//   int    qt_main();           // boucle d'événements Qt
//   void   qt_main_quit();      // demande d’arrêt de la boucle
//
// Build (exemples):
//   g++ -std=c++17 -fPIC -c desktop/qt_real.cpp $(pkg-config --cflags Qt5Widgets) -o build/qt_backend.o
//   g++ build/app.o build/qt_backend.o $(pkg-config --libs Qt5Widgets) -o bin/vitte-desktop
//
//   (Qt6) remplacer Qt5Widgets par Qt6Widgets si nécessaire.
//
// Remarques :
// - Ce backend suppose une UI simple (fenêtre + boutons, etc.).
// - La hiérarchie parent/enfant utilise les layouts verticaux par défaut.

#if defined(_WIN32)
  #define QTREAL_API __declspec(dllexport)
#else
  #define QTREAL_API __attribute__((visibility("default")))
#endif

#include <memory>
#include <algorithm>

#include <QApplication>
#include <QCoreApplication>
#include <QWidget>
#include <QPushButton>
#include <QVBoxLayout>
#include <QString>
#include <QPointer>

namespace qt_real {

static std::unique_ptr<QApplication> g_app;

// Argv factice si qt_init() est appelé sans arguments
static int        s_fake_argc = 1;
static char       s_app0[]    = "vitte-desktop";
static char*      s_fake_argv[] = { s_app0, nullptr };

// S’assure qu’un QApplication existe (utile si qt_main() est appelé sans qt_init()).
static void ensure_app() {
    if (!g_app) {
        g_app.reset(new QApplication(s_fake_argc, s_fake_argv));
    }
}

static QWidget* asWidget(void* p) {
    return reinterpret_cast<QWidget*>(p);
}

static QPushButton* asButton(void* p) {
    return reinterpret_cast<QPushButton*>(p);
}

static void ensure_vbox_layout(QWidget* parent) {
    if (!parent) return;
    if (!parent->layout()) {
        auto* vbox = new QVBoxLayout();
        vbox->setContentsMargins(8, 8, 8, 8);
        vbox->setSpacing(6);
        parent->setLayout(vbox);
    }
}

} // namespace qt_real

extern "C" {

// Initialise l’application Qt (idempotent). argc/argv peuvent être NULL.
QTREAL_API void qt_init(int* argc, char*** argv) {
    if (qt_real::g_app) return;

    if (argc && argv && *argv) {
        // Qt modifie argv/argc (retrait d’arguments). On respecte l’API Qt.
        qt_real::g_app.reset(new QApplication(*argc, *argv));
    } else {
        // Fallback si aucun argument fourni
        qt_real::g_app.reset(new QApplication(qt_real::s_fake_argc, qt_real::s_fake_argv));
    }
}

// Crée une fenêtre top-level (QWidget). Titre/size appliqués si fournis.
QTREAL_API void* qt_window_new(const char* title, int w, int h) {
    qt_real::ensure_app();

    QWidget* win = new QWidget();
    if (title) win->setWindowTitle(QString::fromUtf8(title));
    win->resize(std::max(w, 200), std::max(h, 120));
    // Layout vertical par défaut pour accueillir des enfants
    qt_real::ensure_vbox_layout(win);
    return reinterpret_cast<void*>(win);
}

// Crée un bouton (QPushButton) sans parent initial.
QTREAL_API void* qt_button_new(const char* label) {
    qt_real::ensure_app();

    QString text = label ? QString::fromUtf8(label) : QStringLiteral("Button");
    QPushButton* btn = new QPushButton(text);
    return reinterpret_cast<void*>(btn);
}

// Définit le parent d’un widget et, si parent possède/peut posséder un layout, y ajoute l’enfant.
QTREAL_API void qt_widget_set_parent(void* child, void* parent) {
    QWidget* c = qt_real::asWidget(child);
    QWidget* p = qt_real::asWidget(parent);
    if (!c || !p) return;

    // S’assure d’un layout parent
    qt_real::ensure_vbox_layout(p);

    // Ajouter au layout si possible (setParent est implicite via addWidget)
    if (auto* vbox = qobject_cast<QVBoxLayout*>(p->layout())) {
        vbox->addWidget(c);
    } else {
        c->setParent(p);
    }
}

// Affiche le widget (show()) si c’est un QWidget valide.
QTREAL_API void qt_widget_show(void* widget) {
    QWidget* w = qt_real::asWidget(widget);
    if (!w) return;
    w->show();
}

// Définit le titre :
// - QWidget -> windowTitle
// - QPushButton -> text
QTREAL_API void qt_widget_set_title(void* widget, const char* title) {
    if (!widget) return;
    QString t = title ? QString::fromUtf8(title) : QString();

    // Tente d’abord en tant que bouton
    if (auto* b = qt_real::asButton(widget)) {
        b->setText(t);
        return;
    }
    // Sinon, widget générique
    if (auto* w = qt_real::asWidget(widget)) {
        w->setWindowTitle(t);
    }
}

// Lance la boucle d’événements Qt.
QTREAL_API int qt_main() {
    qt_real::ensure_app();
    return QCoreApplication::exec();
}

// Demande la fin de la boucle (équivalent à QCoreApplication::quit()).
QTREAL_API void qt_main_quit() {
    QCoreApplication::quit();
}

} // extern "C"
