int start_gui(){return 0;} /* link with pkg-config gtk+-3.0 */
// desktop/gtk_stub.c
// Stub GTK — permet la compilation sans la lib GTK

#include <stdio.h>

// Simulation d'initialisation GTK
void gtk_init(int *argc, char ***argv) {
    (void)argc;
    (void)argv;
    fprintf(stderr, "[GTK Stub] gtk_init() ignoré — mode console.\n");
}

// Simulation de création d'une fenêtre
void* gtk_window_new(int type) {
    (void)type;
    fprintf(stderr, "[GTK Stub] gtk_window_new() — pas de fenêtre créée.\n");
    return NULL;
}

// Simulation d’affichage de widget
void gtk_widget_show_all(void *widget) {
    (void)widget;
    fprintf(stderr, "[GTK Stub] gtk_widget_show_all() — aucun rendu.\n");
}

// Simulation de boucle d’événements
void gtk_main(void) {
    fprintf(stderr, "[GTK Stub] gtk_main() ignoré — rien à faire.\n");
}

// Simulation de sortie de boucle
void gtk_main_quit(void) {
    fprintf(stderr, "[GTK Stub] gtk_main_quit() — rien à quitter.\n");
}
