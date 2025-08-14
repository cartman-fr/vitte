int start_qt(){return 0;} // link against QtCore/QtGui
// desktop/qt_stub.cpp
// Stub Qt — compile & run sans Qt, avec traces claires côté stderr.
// But : fournir des symboles C "façon Qt" pour un FFI simple.
//
// API exposée (C ABI):
//   void   qt_init(int *argc, char ***argv);
//   void*  qt_window_new(const char* title, int w, int h);
//   void*  qt_button_new(const char* label);
//   void   qt_widget_set_parent(void* child, void* parent);
//   void   qt_widget_show(void* widget);
//   void   qt_widget_set_title(void* widget, const char* title);
//   int    qt_main();           // boucle d'événements simulée
//   void   qt_main_quit();      // termine la boucle
//
// Design : ne crée **aucune** vraie fenêtre. C’est 100% no-op + logs.

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>
#include <mutex>
#include <atomic>

#if defined(_WIN32)
  #define QTSTUB_API __declspec(dllexport)
#else
  #define QTSTUB_API __attribute__((visibility("default")))
#endif

namespace qtstub {

static std::atomic<bool> g_running{false};
static std::mutex g_mu;
static bool g_verbose = true;

struct Widget {
    enum class Kind { Window, Button, Generic };
    Kind kind = Kind::Generic;
    std::string title;
    std::string label;
    int width = 0;
    int height = 0;
    Widget* parent = nullptr;
    int id = 0;
};

static int next_id = 1;
static std::vector<Widget*> registry;

static void trace(const char* fmt, ...) {
    if (!g_verbose) return;
    va_list ap;
    va_start(ap, fmt);
    std::fprintf(stderr, "[Qt Stub] ");
    std::vfprintf(stderr, fmt, ap);
    std::fprintf(stderr, "\n");
    va_end(ap);
}

static Widget* make_widget(Widget::Kind k) {
    std::lock_guard<std::mutex> lock(g_mu);
    auto* w = new Widget();
    w->kind = k;
    w->id = next_id++;
    registry.push_back(w);
    return w;
}

static const char* kind_name(Widget::Kind k) {
    switch (k) {
        case Widget::Kind::Window: return "Window";
        case Widget::Kind::Button: return "Button";
        default: return "Widget";
    }
}

static void dump_widget(const Widget* w, const char* prefix) {
    trace("%s #%d kind=%s title='%s' label='%s' size=%dx%d parent=#%d",
          prefix,
          w->id,
          kind_name(w->kind),
          w->title.c_str(),
          w->label.c_str(),
          w->width, w->height,
          w->parent ? w->parent->id : 0);
}

} // namespace qtstub

extern "C" {

// Active/force le verbosité via env QT_STUB_VERBOSE=0/1
QTSTUB_API void qt_set_verbose(int on) {
    qtstub::g_verbose = (on != 0);
}

// argc/argv ignorés mais gardés pour signature compatible
QTSTUB_API void qt_init(int* argc, char*** argv) {
    (void)argc; (void)argv;
    const char* v = std::getenv("QT_STUB_VERBOSE");
    if (v) qtstub::g_verbose = (std::strcmp(v, "0") != 0);
    qtstub::trace("qt_init() — mode console (aucune GUI réelle).");
}

// Crée une “fenêtre” logique (pas de rendu)
QTSTUB_API void* qt_window_new(const char* title, int w, int h) {
    auto* win = qtstub::make_widget(qtstub::Widget::Kind::Window);
    win->title  = title ? title : "";
    win->width  = (w > 0) ? w : 800;
    win->height = (h > 0) ? h : 600;
    qtstub::dump_widget(win, "window_new");
    return (void*)win;
}

// Crée un “bouton” logique
QTSTUB_API void* qt_button_new(const char* label) {
    auto* btn = qtstub::make_widget(qtstub::Widget::Kind::Button);
    btn->label = label ? label : "Button";
    qtstub::dump_widget(btn, "button_new");
    return (void*)btn;
}

// Parentage logique (layout non géré)
QTSTUB_API void qt_widget_set_parent(void* child, void* parent) {
    if (!child) return;
    auto* c = reinterpret_cast<qtstub::Widget*>(child);
    auto* p = reinterpret_cast<qtstub::Widget*>(parent);
    c->parent = p;
    qtstub::dump_widget(c, "set_parent");
}

// Show (trace uniquement)
QTSTUB_API void qt_widget_show(void* widget) {
    if (!widget) {
        qtstub::trace("widget_show(NULL) — ignoré.");
        return;
    }
    auto* w = reinterpret_cast<qtstub::Widget*>(widget);
    qtstub::dump_widget(w, "widget_show");
}

// Titre (utile pour Window)
QTSTUB_API void qt_widget_set_title(void* widget, const char* title) {
    if (!widget) return;
    auto* w = reinterpret_cast<qtstub::Widget*>(widget);
    w->title = title ? title : "";
    qtstub::dump_widget(w, "set_title");
}

// Boucle d’événements simulée
QTSTUB_API int qt_main() {
    qtstub::trace("qt_main() — début boucle simulée.");
    qtstub::g_running.store(true);
    // Boucle très simple : on dort par petits pas jusqu’à quit()
    while (qtstub::g_running.load()) {
        // Ici on pourrait simuler des timers/événements…
        #if defined(_WIN32)
            Sleep(16);
        #else
            struct timespec ts{0, 16 * 1000 * 1000};
            nanosleep(&ts, nullptr);
        #endif
    }
    qtstub::trace("qt_main() — fin boucle simulée.");
    return 0;
}

QTSTUB_API void qt_main_quit() {
    qtstub::trace("qt_main_quit() — demande d’arrêt.");
    qtstub::g_running.store(false);
}

} // extern "C"
