# Vitte Language — Modern Systems Programming Language

<p align="center">
  <img src="assets/logo-vitte.svg" alt="Vitte Logo" width="220">
</p>

<p align="center">
  <i>Rapide comme C++, sûr comme Rust, simple comme Go — et prêt pour le futur.</i>
</p>

<p align="center">
  <a href="https://github.com/vitte-lang/vitte/actions">
    <img src="https://img.shields.io/github/actions/workflow/status/vitte-lang/vitte/ci.yml?branch=main&label=build&style=flat-square">
  </a>
  <a href="https://github.com/vitte-lang/vitte/releases">
    <img src="https://img.shields.io/github/v/release/vitte-lang/vitte?style=flat-square">
  </a>
  <img src="https://img.shields.io/github/downloads/vitte-lang/vitte/total?style=flat-square">
  <img src="https://img.shields.io/codecov/c/github/vitte-lang/vitte?style=flat-square">
  <img src="https://img.shields.io/github/license/vitte-lang/vitte?style=flat-square">
  <img src="https://img.shields.io/github/languages/top/vitte-lang/vitte?style=flat-square">
</p>

---

## 📖 Sommaire

1. [Présentation](#-présentation)
2. [Pourquoi Vitte ?](#-pourquoi-vitte-)
3. [Comparatif avec autres langages](#-comparatif-avec-autres-langages)
4. [Architecture interne](#-architecture-interne)
5. [Compatibilité & Support](#-compatibilité--support)
6. [Download & Installation](#-download--installation)
7. [Tutoriel complet](#-tutoriel-complet)
8. [Exemples avancés](#-exemples-avancés)
9. [Syntaxe essentielle](#-syntaxe-essentielle)
10. [Cas d’usage](#-cas-dusage)
11. [Roadmap](#-roadmap)
12. [Références & Documentation](#-références--documentation)
13. [Licence](#-licence)

---

## 🌟 Présentation

**Vitte** est un langage de programmation **systèmes et applicatif** pensé pour l’ère post-2025.  
Il allie **performance**, **sécurité mémoire** et **expressivité**, avec une philosophie claire :  
> _"Un langage doit vous permettre de coder vite, bien, et longtemps."_

---

## 💡 Pourquoi Vitte ?

- **Sûreté** : pas de _null_ implicite, borrow-checker intégré.
- **Performance brute** : compilation en code machine optimisé via LLVM/Cranelift.
- **Simplicité syntaxique** : inspirée de C++, Rust, et C#, mais plus concise.
- **Interop totale** : C, C++, Rust, Zig, ASM.
- **Portabilité** : x86_64, ARM, RISC-V, WebAssembly.

---

## ⚔ Comparatif avec autres langages

| Critère                  | Vitte       | Rust        | C++17/20   | Go       | Zig      |
|--------------------------|-------------|-------------|------------|----------|----------|
| Sécurité mémoire         | ✅ Borrow-checker | ✅ Borrow-checker | ❌ (manuelle) | ✅ GC | ✅ Opt-in safety |
| Backend                  | LLVM / Cranelift / VM | LLVM | Varié | Custom | LLVM |
| Concurrency              | ✅ Threads + async/await | ✅ | Partiel | ✅ Goroutines | ✅ |
| Cross-compilation        | ✅ intégrée | ✅ | ❌ | ✅ | ✅ |
| Package manager          | ✅ `vitpm` intégré | ✅ Cargo | ❌ | ✅ go mod | ❌ |
| Syntaxe                  | Simple & claire | Complexe | Verbose | Minimaliste | Minimaliste |

---

## 🛠 Architecture interne

```
          ┌──────────────┐
          │   Source .vit│
          └──────┬───────┘
                 ▼
       ┌─────────────────┐
       │ Frontend         │
       │ - Lexer          │
       │ - Parser         │
       │ - Borrow Checker │
       └────────┬────────┘
                ▼
       ┌─────────────────┐
       │ Middle-end       │
       │ - IR Optimisation│
       │ - Inlining       │
       │ - Flow Analysis  │
       └────────┬────────┘
                ▼
   ┌────────────────────────┐
   │ Backend                 │
   │ - LLVM (opt)            │
   │ - Cranelift (JIT)       │
   │ - VitteVM (bytecode)    │
   └──────────┬─────────────┘
              ▼
         Exécutable binaire
```

---

## 🖥 Compatibilité & Support

| OS / Arch             | Support |
|-----------------------|---------|
| Linux x86_64          | ✅ Stable |
| Linux ARM64           | ✅ Stable |
| macOS Intel           | ✅ Stable |
| macOS Apple Silicon   | ✅ Stable |
| Windows 10/11 x64     | ✅ Stable |
| FreeBSD / OpenBSD     | 🧪 Expérimental |
| RISC-V                | 🧪 Expérimental |
| WebAssembly           | 🚧 En cours |

---

## 📥 Download & Installation

### Binaires officiels
- [📦 Dernière version stable (GitHub Releases)](https://github.com/vitte-lang/vitte/releases/latest)
- [🌙 Version nightly](https://github.com/vitte-lang/vitte/releases/tag/nightly)

### Installation via script
```sh
curl -sSL https://get.vitte.dev | sh
```

### Installation via package managers
```sh
# Arch Linux
pacman -S vitte

# macOS (Homebrew)
brew install vitte

# Windows (Scoop)
scoop install vitte
```

### Docker
```sh
docker run -it vitte/vitte:latest
```

### Compilation depuis les sources
```sh
git clone https://github.com/vitte-lang/vitte.git
cd vitte
cargo build --release
./target/release/vitc --version
```

---

## 🚀 Tutoriel complet

### 1. Hello World
```vitte
do main() {
    print("Hello, Vitte!")
}
```

### 2. Projet structuré
```
my_app/
├── src/
│   ├── main.vit
│   └── utils.vit
├── vitte.toml
└── tests/
    └── test_main.vit
```

### 3. Compilation & exécution
```sh
vitc src/main.vit -o my_app
./my_app
```

### 4. Cross-compilation
```sh
vitc src/main.vit --target x86_64-pc-windows-gnu -o app.exe
```

---

## 🔬 Exemples avancés

### Pattern Matching
```vitte
match get_data() {
    Ok(val) => print(val),
    Err(e)  => print("Erreur: " + e)
}
```

### Async
```vitte
async do fetch() {
    await net::get("https://vitte.dev")
}
```

### FFI C
```vitte
extern(c) do printf(fmt: *u8, ...) -> i32
do main() {
    printf("Nombre: %d\n", 42)
}
```

---

## 📚 Syntaxe essentielle

| Élément         | Exemple |
|-----------------|---------|
| Déclaration     | `let x = 10` |
| Fonction        | `do add(a: i32, b: i32) -> i32 { a + b }` |
| Condition       | `if x > 0 { ... } else { ... }` |
| Boucle          | `for i in 0..10 { ... }` |
| Match           | `match val { ... }` |
| Import          | `use std::io` |

---

## 🗺 Roadmap

- [x] Backend LLVM + Cranelift
- [x] Borrow-checker natif
- [ ] Débogueur intégré
- [ ] IDE Vitte Studio
- [ ] Support WebAssembly complet
- [ ] Bibliothèque standard étendue

---

## 📖 Références & Documentation

- [Documentation Officielle](https://vitte.dev/docs)
- [Standard Library](https://vitte.dev/std)
- [RFCs du langage](https://vitte.dev/rfcs)
- [Guide Contributeurs](CONTRIBUTING.md)

---

## 📜 Licence

Double licence :
- MIT
- Apache 2.0

Voir [LICENSE-MIT](LICENSE-MIT) et [LICENSE-APACHE](LICENSE-APACHE).
