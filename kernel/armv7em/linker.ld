/* kernel/armv7em/linker.ld
 *
 * Linker script générique ARMv7-M/EM (Cortex-M4/M7).
 * Par défaut : FLASH @ 0x0800_0000 (512K), RAM @ 0x2000_0000 (128K).
 * Adapte ORIGIN/LENGTH selon ta carte (ou passe -Wl,-defsym=...).
 */

/* ———————————————————————————————————————————————
   Paramètres override-ables via -Wl,-defsym=SYMBOL=VALUE
   ——————————————————————————————————————————————— */
STACK_SIZE      = DEFINED(STACK_SIZE)      ? STACK_SIZE      : 0x1000;   /* 4 KiB */
MIN_HEAP_SIZE   = DEFINED(MIN_HEAP_SIZE)   ? MIN_HEAP_SIZE   : 0x0000;   /* min heap */
VECT_ALIGN      = DEFINED(VECT_ALIGN)      ? VECT_ALIGN      : 256;      /* VTOR align */
FLASH_ORIGIN    = DEFINED(FLASH_ORIGIN)    ? FLASH_ORIGIN    : 0x08000000;
FLASH_LENGTH    = DEFINED(FLASH_LENGTH)    ? FLASH_LENGTH    : 512K;
RAM_ORIGIN      = DEFINED(RAM_ORIGIN)      ? RAM_ORIGIN      : 0x20000000;
RAM_LENGTH      = DEFINED(RAM_LENGTH)      ? RAM_LENGTH      : 128K;

/* ———————————————————————————————————————————————
   Mémoire
   ——————————————————————————————————————————————— */
MEMORY
{
  FLASH (rx)  : ORIGIN = FLASH_ORIGIN, LENGTH = FLASH_LENGTH
  RAM   (rwx) : ORIGIN = RAM_ORIGIN,   LENGTH = RAM_LENGTH
}

/* Point d’entrée : Reset_Handler (défini dans kmain.vitte) */
ENTRY(Reset_Handler)

/* ———————————————————————————————————————————————
   SECTIONS
   ——————————————————————————————————————————————— */
SECTIONS
{
  /* Table des vecteurs au tout début de la FLASH.
     Accepte .isr_vector et .vectors (selon ton code). */
  .vectors ORIGIN(FLASH) :
  {
    . = ALIGN(VECT_ALIGN);
    KEEP(*(.isr_vector))             /* startup C, si utilisé */
    KEEP(*(.isr_vector.*))
    KEEP(*(.vectors))                /* table statique en Vitte */
    KEEP(*(.vectors.*))
    . = ALIGN(4);
  } > FLASH

  /* Code + constantes */
  .text :
  {
    . = ALIGN(4);
    *(.text.Reset_Handler)           /* si présent dans C */
    *(.text*)                        /* code */
    *(.glue_7) *(.glue_7t)
    *(.gnu.linkonce.t.*)
    . = ALIGN(4);

    /* Tables d’exception/unwind (optionnelles mais inoffensives) */
    KEEP(*(.init))
    KEEP(*(.fini))
    . = ALIGN(4);
  } > FLASH

  .rodata :
  {
    . = ALIGN(4);
    *(.rodata*)                      /* constantes */
    *(.gnu.linkonce.r.*)
    . = ALIGN(4);
  } > FLASH

  /* Unwind (nécessaire si tu relies du C++/libgcc avec EH) */
  .ARM.extab :
  {
    *(.ARM.extab* .gnu.linkonce.armextab.*)
  } > FLASH
  __exidx_start = .;
  .ARM.exidx :
  {
    *(.ARM.exidx* .gnu.linkonce.armexidx.*)
  } > FLASH
  __exidx_end = .;

  /* Tables d’init/fini (constructeurs C/C++) */
  .preinit_array :
  {
    PROVIDE_HIDDEN (__preinit_array_start = .);
    KEEP(*(.preinit_array*))
    PROVIDE_HIDDEN (__preinit_array_end = .);
  } > FLASH

  .init_array :
  {
    PROVIDE_HIDDEN (__init_array_start = .);
    KEEP(*(SORT(.init_array.*)))
    KEEP(*(.init_array*))
    PROVIDE_HIDDEN (__init_array_end = .);
  } > FLASH

  .fini_array :
  {
    PROVIDE_HIDDEN (__fini_array_start = .);
    KEEP(*(SORT(.fini_array.*)))
    KEEP(*(.fini_array*))
    PROVIDE_HIDDEN (__fini_array_end = .);
  } > FLASH

  /* ——————————————————————————————
     Données initialisées en RAM (load en FLASH)
     —————————————————————————————— */
  .data : AT (LOADADDR(.text) + SIZEOF(.text) + SIZEOF(.rodata) + SIZEOF(.ARM.extab) + SIZEOF(.ARM.exidx) + SIZEOF(.preinit_array) + SIZEOF(.init_array) + SIZEOF(.fini_array))
  {
    . = ALIGN(4);
    _sdata = .;                      /* début data en RAM */
    *(.data*)
    *(.gnu.linkonce.d.*)
    . = ALIGN(4);
    _edata = .;                      /* fin data en RAM */
  } > RAM
  /* Adresse de chargement en FLASH (pour copie .data au reset) */
  _sidata = LOADADDR(.data);

  /* Code/exécutables en RAM optionnels (si tu as .ramfunc) */
  .ramfunc (NOLOAD) :
  {
    . = ALIGN(4);
    *(.ramfunc*)
    . = ALIGN(4);
  } > RAM

  /* ——————————————————————————————
     BSS (zero-init en RAM)
     —————————————————————————————— */
  .bss (NOLOAD) :
  {
    . = ALIGN(4);
    _sbss = .;                       /* début bss */
    *(.bss*)
    *(.gnu.linkonce.b.*)
    *(COMMON)
    . = ALIGN(4);
    _ebss = .;                       /* fin bss */
  } > RAM

  /* ——————————————————————————————
     Heap & Stack — bornes symboliques
     —————————————————————————————— */

  /* Aligner la fin de BSS */
  . = ALIGN(8);
  _heap_start = .;                   /* début heap juste après bss */

  /* Réserver l’espace stack en haut de RAM */
  __StackTop   = ORIGIN(RAM) + LENGTH(RAM);
  __StackLimit = __StackTop - STACK_SIZE;

  /* Taille minimale de heap (optionnelle) */
  _heap_end = (__StackLimit > (_heap_start + MIN_HEAP_SIZE))
            ? (__StackLimit)
            : (_heap_start + MIN_HEAP_SIZE);

  /* Symboles legacy attendus par ton runtime */
  PROVIDE(_estack = __StackTop);

  /* Débogage : vérifie qu’on n’écrase pas la stack avec le .bss */
  ASSERT(_heap_start <= __StackLimit, "Mémoire RAM insuffisante: BSS/HEAP empiète sur STACK");

  /* End of image (utile pour dumps/map) */
  _image_start_flash = ORIGIN(FLASH);
  _image_end_flash   = LOADADDR(.data) + SIZEOF(.data);
  _image_start_ram   = ORIGIN(RAM);
  _image_end_ram     = _heap_end;
}

/* ———————————————————————————————————————————————
   Cartographie symbolique additionnelle (compat.)
   ——————————————————————————————————————————————— */
PROVIDE(_ebss_end = _ebss);
PROVIDE(__heap_start__ = _heap_start);
PROVIDE(__heap_end__   = _heap_end);

/* ———————————————————————————————————————————————
   Notes d’usage
   - Adapter FLASH/RAM via -Wl,-defsym=FLASH_LENGTH=1024K etc.
   - Adapter STACK_SIZE/MIN_HEAP_SIZE selon ton besoin.
   - La table des vecteurs doit être émise en .isr_vector ou .vectors.
   - Le startup (Reset_Handler) copie .data depuis _sidata → [_sdata.._edata]
     et met à zéro [_sbss.._ebss], comme ton kmain.vitte le fait déjà.
   ——————————————————————————————————————————————— */
