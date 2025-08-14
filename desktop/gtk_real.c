// desktop/gtk_real.c
// Backend GTK "réel" pour Vitte — utilise GTK 3 (Widgets) et expose une API C stable:
//   void   vgtk_init(int *argc, char ***argv);
//   void*  vgtk_window_new(const char* title, int w, int h);
//   void*  vgtk_button_new(const char* label);
//   void   vgtk_widget_set_parent(void* child, void* parent);
//   void   vgtk_widget_set_title(void* widget, const char* title);
//   void   vgtk_widget_show(void* widget);
//   void   vgtk_widget_show_all(void* widget);
//   int    vgtk_main(void);
//   void   vgtk_main_quit(void);
//
// Remarque importante : on évite DÉLIBÉRÉMENT de définir des symboles nommés
// "gtk_*" pour ne pas entrer en collision avec la vraie lib GTK au link.
//
// Build (exemples) :
//   cc -O2 -fPIC -c desktop/gtk_real.c $(pkg-config --cflags gtk+-3.0) -o build/gtk_backend.o
//   cc build/app.o build/gtk_backend.o $(pkg-config --libs gtk+-3.0) -o bin/vitte-desktop

#if defined(_WIN32)
#  define VGTK_API __declspec(dllexport)
#else
#  define VGTK_API __attribute__((visibility("default")))
#endif

#include <gtk/gtk.h>
#include <stdio.h>

static gboolean on_window_delete(GtkWidget* widget, GdkEvent* event, gpointer user_data) {
    (void)widget; (void)event; (void)user_data;
    gtk_main_quit();
    return TRUE; // on gère la fermeture
}

static gboolean on_button_clicked(GtkWidget* btn, gpointer user_data) {
    (void)btn;
    const char* what = (const char*)user_data;
    if (what && g_strcmp0(what, "quit") == 0) {
        gtk_main_quit();
    } else {
        g_message("[GTK] Button clicked: %s", what ? what : "(no label)");
    }
    return FALSE;
}

VGTK_API void vgtk_init(int *argc, char ***argv) {
    // Idempotent côté GTK, mais on peut tester :
    if (!gtk_init_check(argc, argv)) {
        fprintf(stderr, "[GTK] gtk_init_check() a échoué (affichage non disponible?)\n");
        // On laisse continuer: à toi de fallback en CLI côté Vitte si besoin
    }
}

VGTK_API void* vgtk_window_new(const char* title, int w, int h) {
    GtkWidget* win = gtk_window_new(GTK_WINDOW_TOPLEVEL);
    if (title) gtk_window_set_title(GTK_WINDOW(win), title);
    if (w <= 0) w = 800;
    if (h <= 0) h = 600;
    gtk_window_set_default_size(GTK_WINDOW(win), w, h);

    // Conteneur vertical par défaut (comme pour Qt)
    GtkWidget* vbox = gtk_box_new(GTK_ORIENTATION_VERTICAL, 6);
    gtk_container_add(GTK_CONTAINER(win), vbox);

    // Fermer proprement
    g_signal_connect(win, "delete-event", G_CALLBACK(on_window_delete), NULL);
    return (void*)win;
}

VGTK_API void* vgtk_button_new(const char* label) {
    GtkWidget* btn = gtk_button_new_with_label(label ? label : "Button");
    // Par défaut, on connecte un petit handler générique
    g_signal_connect(btn, "clicked", G_CALLBACK(on_button_clicked),
                     (gpointer)(label ? label : "Button"));
    return (void*)btn;
}

VGTK_API void vgtk_widget_set_parent(void* child, void* parent) {
    if (!child || !parent) return;
    GtkWidget* c = (GtkWidget*)child;
    GtkWidget* p = (GtkWidget*)parent;

    // Si parent a un GtkBox (notre vbox par défaut), on y ajoute l’enfant
    if (GTK_IS_CONTAINER(p)) {
        // Cherche un conteneur direct (si Window: son unique enfant est notre vbox)
        if (GTK_IS_WINDOW(p)) {
            GList* list = gtk_container_get_children(GTK_CONTAINER(p));
            GtkWidget* first = list ? GTK_WIDGET(list->data) : NULL;
            if (first && GTK_IS_BOX(first)) {
                gtk_box_pack_start(GTK_BOX(first), c, FALSE, FALSE, 0);
                g_list_free(list);
                return;
            }
            if (list) g_list_free(list);
        }
        // Sinon on tente un pack direct dans un box, ou add générique
        if (GTK_IS_BOX(p)) {
            gtk_box_pack_start(GTK_BOX(p), c, FALSE, FALSE, 0);
        } else {
            gtk_container_add(GTK_CONTAINER(p), c);
        }
    }
}

VGTK_API void vgtk_widget_set_title(void* widget, const char* title) {
    if (!widget) return;
    GtkWidget* w = (GtkWidget*)widget;
    if (GTK_IS_WINDOW(w)) {
        gtk_window_set_title(GTK_WINDOW(w), title ? title : "");
    } else if (GTK_IS_BUTTON(w)) {
        gtk_button_set_label(GTK_BUTTON(w), title ? title : "");
    }
}

VGTK_API void vgtk_widget_show(void* widget) {
    if (!widget) return;
    gtk_widget_show(GTK_WIDGET(widget));
}

VGTK_API void vgtk_widget_show_all(void* widget) {
    if (!widget) return;
    gtk_widget_show_all(GTK_WIDGET(widget));
}

VGTK_API int vgtk_main(void) {
    gtk_main();
    return 0;
}

VGTK_API void vgtk_main_quit(void) {
    gtk_main_quit();
}
