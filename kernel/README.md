# Kernel skeleton (placeholder)
linker.ld, start.S, main.vitte
# Kernel — Documentation Complète

Ce dossier contient le noyau bas-niveau pour deux architectures :
- **ARMv7EM** (ex. Cortex-M4/M7)
- **x86_64** (PC/serveur avec chargeur Multiboot2)

## 📂 Structure du dossier

```
kernel/
├── armv7em/
│   ├── kmain.vitte     # Point d'entrée kernel (Vitte, bare-metal, no_std)
│   ├── linker.ld       # Script de linkage pour ARMv7EM
│   └── start.S         # Startup code & table des vecteurs
│
├── x86_64/
│   ├── kmain.vitte     # Point d'entrée kernel (Vitte, no_std, long mode)
│   ├── linker.ld       # Script de linkage kernel 64-bit
│   └── start.S         # Bootstrap Multiboot2 + Long Mode + ISRs
│
└── README.md           # Ce fichier
```

## 🔧 Toolchains nécessaires

### Pour ARMv7EM
- **Compilateur Vitte** capable de générer du code ARMv7-M/EM (`--target thumbv7em-none-eabi`)
- `arm-none-eabi-gcc` et `arm-none-eabi-ld` pour l'assemblage/linkage
- `arm-none-eabi-objcopy` pour générer le binaire `.bin` flashable

### Pour x86_64
- **Compilateur Vitte** (`--target x86_64-unknown-none`)
- `x86_64-elf-gcc` et `x86_64-elf-ld` pour l'assemblage/linkage
- `grub-mkrescue` (pour créer une ISO bootable via GRUB2)
- Optionnel : [Limine](https://limine-bootloader.org/) pour un bootloader moderne

## 🚀 Compilation

### ARMv7EM
```bash
cd kernel/armv7em
vittec --target thumbv7em-none-eabi -c kmain.vitte -o kmain.o
arm-none-eabi-as start.S -o start.o
arm-none-eabi-ld -T linker.ld -o kernel.elf start.o kmain.o
arm-none-eabi-objcopy -O binary kernel.elf kernel.bin
```

### x86_64
```bash
cd kernel/x86_64
vittec --target x86_64-unknown-none -c kmain.vitte -o kmain.o
x86_64-elf-gcc -m64 -ffreestanding -c start.S -o start64.o
x86_64-elf-ld -T linker.ld -o kernel.elf start64.o kmain.o
```

#### Création d’une ISO bootable avec GRUB2
```bash
mkdir -p iso/boot/grub
cp kernel.elf iso/boot/kernel.elf
echo 'set timeout=0
set default=0
menuentry "Vitte Kernel" {
    multiboot2 /boot/kernel.elf
    boot
}' > iso/boot/grub/grub.cfg
grub-mkrescue -o kernel.iso iso
```

## 🧠 Processus de Boot

### ARMv7EM
1. **Reset vector** (défini dans `start.S`) initialise la pile et appelle `Reset_Handler`.
2. `.data` et `.bss` sont initialisées.
3. Appel de `kmain()` en Vitte.
4. Boucle principale ou gestion des IRQ.

### x86_64
1. Le bootloader charge `kernel.elf` en mémoire.
2. Exécution de `_start` dans `start.S` :
   - Passage en Long Mode
   - Initialisation de la pile
   - Nettoyage `.bss`
   - Saut vers `kmain()`
3. Gestion des interruptions et drivers basiques.

## 🗺️ Mémoire et Sections

| Section     | Description |
|-------------|-------------|
| `.text`     | Code exécutable |
| `.rodata`   | Données constantes |
| `.data`     | Données initialisées |
| `.bss`      | Données non initialisées (zéro) |
| `.isr_vector` / `.multiboot2` | Table vecteurs ARM / Header Multiboot2 |
| `.stack`    | Zone de pile initiale |

## 🔮 Extensions futures

- Support SMP sur x86_64 (AP bring-up)
- Gestion d’exceptions plus fine sur ARM (HardFault, MemManage)
- Intégration d’un mini-driver série pour debug
- Ajout d’un allocateur mémoire bas-niveau
- Abstraction des timers pour portable HAL

---
© 2025 Projet Vitte Kernel — Licence MIT/Apache-2.0
