# Vitte Desktop — Interfaces GTK et Qt

Ce dossier contient le code et les backends graphiques pour **Vitte Desktop**.  
L'objectif est de permettre l'exécution d'applications graphiques **ou** de proposer un **mode console de secours** lorsque le toolkit graphique n'est pas disponible.

---

## 🎯 Objectifs
- **Interface multiplateforme** (GTK, Qt, CLI fallback)
- **Compilation flexible** (backend réel ou stub)
- **FFI clair** pour liaison entre Vitte et C/C++
- **Facilité de debug** grâce aux logs des stubs
- **Compatibilité CI/CD**

---

## 📂 Structure

```
desktop/
│
├── main.vitte        # Point d'entrée Vitte (sélection auto CLI/GUI)
│
├── gtk_real.c        # Backend réel GTK (liens avec libgtk-3/4)
├── gtk_stub.c        # Stub GTK (impl. minimale sans dépendance)
│
├── qt_real.cpp       # Backend réel Qt (Qt5/6 via wrappers C)
├── qt_stub.cpp       # Stub Qt (impl. minimale, logs configurables)
│
├── CMakeLists.txt    # (optionnel) Build CMake multiplateforme
├── Makefile          # Build GNU Make avec sélection backend
│
└── README.md         # Documentation
```

---

## ⚙️ Compilation

### 1. GTK Réel

Installer GTK3 :

```sh
sudo apt install libgtk-3-dev   # Debian/Ubuntu
brew install gtk+3              # macOS (Homebrew)
```

Compiler :

```sh
make USE_GTK=1
```

### 2. GTK Stub

Pas besoin d'installer GTK :

```sh
make USE_GTK=0
```

---

### 3. Qt Réel

Installer Qt5 ou Qt6 :

```sh
sudo apt install qtbase5-dev
```

Compiler :

```sh
make USE_QT=1
```

### 4. Qt Stub

```sh
make USE_QT=0
```

---

## 🔌 FFI (Foreign Function Interface)

Les backends exposent des symboles C simples :

```c
// GTK
void gtk_init(int *argc, char ***argv);
void* gtk_window_new(int type);
void gtk_widget_show_all(void* widget);
void gtk_main(void);
void gtk_main_quit(void);

// Qt
void qt_init(int *argc, char ***argv);
void* qt_window_new(const char* title, int w, int h);
void qt_widget_show(void* widget);
int qt_main(void);
void qt_main_quit(void);
```

Ces fonctions sont importées côté Vitte avec `extern(c)` :

```vitte
extern(c) {
    fn gtk_init(argc: *int, argv: **char);
    fn gtk_main();
}
```

---

## 🖥 Exemple `main.vitte` (simplifié)

```vitte
fn main(args: [str]) -> int {
    gtk_init(null, null);
    let win = gtk_window_new(0);
    gtk_widget_show_all(win);
    gtk_main();
    return 0;
}
```

---

## 🔄 Cross-compilation

Pour cibler une autre plateforme sans toolkit graphique :

```sh
make USE_GTK=0 TARGET=windows
```

Les stubs évitent tout besoin de dépendances natives.

---

## 🐞 Debug

- **GTK Stub** : trace chaque appel sur `stderr`.
- **Qt Stub** : activer/désactiver avec `QT_STUB_VERBOSE=1` ou `0`.

---

## 📌 Pourquoi utiliser des stubs ?

- **Dev rapide** : pas besoin d’installer GTK/Qt.
- **CI/CD** : pas de dépendance lourde pour exécuter les tests.
- **Fallback console** : interface minimale quand GUI indisponible.
- **Log clair** : suivi des appels UI.

---

## 📜 TODO

- Implémenter les backends réels.
- Ajouter un bus d’événements commun GUI/CLI.
- Simuler signaux/slots dans les stubs Qt.
- Ajouter un mode `--headless` global.
