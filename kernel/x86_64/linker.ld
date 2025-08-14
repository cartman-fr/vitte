
ENTRY(_start)
SECTIONS {
  . = 1M;
  .text : { *(.text*) }
  .rodata : { *(.rodata*) }
  .data : { *(.data*) }
  .bss : { *(.bss*) *(COMMON) }
}
/* x86_64/linker.ld
 * Linker script générique kernel x86_64 (ELF64).
 * - Entry  : _start (fourni par start64.S)
 * - Par défaut: VMA = LMA = 0x00100000 (1 MiB)
 * - Option HHK: KERNEL_VMA=0xffffffff80000000, KERNEL_LMA=0x00100000
 */

OUTPUT_FORMAT("elf64-x86-64")
OUTPUT_ARCH(i386:x86-64)
ENTRY(_start)

/* -----------------------------
   Paramètres override-ables
   ----------------------------- */
KERNEL_VMA   = DEFINED(KERNEL_VMA) ? KERNEL_VMA : 0x00100000;      /* Virt addr */
KERNEL_LMA   = DEFINED(KERNEL_LMA) ? KERNEL_LMA : KERNEL_VMA;      /* Phys addr / load addr */
PAGE_SIZE    = DEFINED(PAGE_SIZE)  ? PAGE_SIZE  : 0x1000;
STACK_SIZE   = DEFINED(STACK_SIZE) ? STACK_SIZE : 0x4000;          /* 16 KiB */

PHYS_OFFSET  = KERNEL_LMA - KERNEL_VMA;

/* Quelques symboles publics utiles */
PROVIDE(__kernel_vma   = KERNEL_VMA);
PROVIDE(__kernel_lma   = KERNEL_LMA);
PROVIDE(__phys_offset  = PHYS_OFFSET);
PROVIDE(__page_size    = PAGE_SIZE);
PROVIDE(__stack_size   = STACK_SIZE);

SECTIONS
{
  /* Positionne le compteur à la VMA du kernel */
  . = KERNEL_VMA;

  /* -----------------------------
     En-tête Multiboot2 (si tu as .multiboot2)
     ----------------------------- */
  .multiboot2 ALIGN(8) : AT(ADDR(.multiboot2) + PHYS_OFFSET)
  {
    KEEP(*(.multiboot2))
  }

  /* -----------------------------
     Code
     ----------------------------- */
  .text ALIGN(PAGE_SIZE) : AT(ADDR(.text) + PHYS_OFFSET)
  {
    __text_start = .;
    KEEP(*(.text.boot))                 /* code de bootstrap si tu en as */
    *(.text .text.*)
    *(.gnu.linkonce.t.*)
    __text_end = .;
  }

  /* -----------------------------
     Constantes en lecture seule
     ----------------------------- */
  .rodata ALIGN(PAGE_SIZE) : AT(ADDR(.rodata) + PHYS_OFFSET)
  {
    __rodata_start = .;
    *(.rodata .rodata.*)
    *(.gnu.linkonce.r.*)
    __rodata_end = .;
  }

  /* Unwind (eh_frame) — inoffensif si non utilisé */
  .eh_frame_hdr ALIGN(8) : AT(ADDR(.eh_frame_hdr) + PHYS_OFFSET)
  { *(.eh_frame_hdr) *(.eh_frame_hdr.*) }

  .eh_frame ALIGN(8) : AT(ADDR(.eh_frame) + PHYS_OFFSET)
  { KEEP(*(.eh_frame)) KEEP(*(.eh_frame.*)) }

  /* -----------------------------
     Constructeurs/Destructeurs C++
     ----------------------------- */
  .init_array ALIGN(8) : AT(ADDR(.init_array) + PHYS_OFFSET)
  {
    PROVIDE(__init_array_start = .);
    KEEP(*(SORT_BY_INIT_PRIORITY(.init_array.*)))
    KEEP(*(.init_array))
    PROVIDE(__init_array_end = .);
  }

  .fini_array ALIGN(8) : AT(ADDR(.fini_array) + PHYS_OFFSET)
  {
    PROVIDE(__fini_array_start = .);
    KEEP(*(SORT_BY_INIT_PRIORITY(.fini_array.*)))
    KEEP(*(.fini_array))
    PROVIDE(__fini_array_end = .);
  }

  /* Compat (certains toolchains émettent encore .ctors/.dtors) */
  .ctors ALIGN(8) : AT(ADDR(.ctors) + PHYS_OFFSET)
  { KEEP(*(SORT(.ctors.*))) KEEP(*(.ctors)) }
  .dtors ALIGN(8) : AT(ADDR(.dtors) + PHYS_OFFSET)
  { KEEP(*(SORT(.dtors.*))) KEEP(*(.dtors)) }

  /* -----------------------------
     Données initialisées
     ----------------------------- */
  .data ALIGN(PAGE_SIZE) : AT(ADDR(.data) + PHYS_OFFSET)
  {
    __data_start = .;
    *(.data .data.*)
    *(.gnu.linkonce.d.*)
    __data_end = .;
  }

  /* -----------------------------
     BSS (zéro-init)
     ----------------------------- */
  .bss ALIGN(PAGE_SIZE) (NOLOAD) :
  {
    __bss_start = .;
    *(.bss .bss.*)
    *(COMMON)
    __bss_end = .;
  }

  /* -----------------------------
     Stack du BSP (noyau)
     ----------------------------- */
  .bss.stack ALIGN(16) (NOLOAD) :
  {
    __stack_bottom = .;
    . += STACK_SIZE;
    __stack_top = .;
  }

  /* Marqueurs de bornes du kernel */
  __kernel_start = KERNEL_VMA;
  __kernel_end   = .;

  /* Jette le bruit inutile */
  /DISCARD/ :
  {
    *(.comment) *(.note.GNU-stack)
    *(.gnu_debuglink) *(.gnu.lto_*) *(.note.*) *(.eh_frame_hdr.*)
  }
}

/* Alias conviviaux */
PROVIDE(_kernel_start = __kernel_start);
PROVIDE(_kernel_end   = __kernel_end);
PROVIDE(_bss_start    = __bss_start);
PROVIDE(_bss_end      = __bss_end);
PROVIDE(_stack_top    = __stack_top);
PROVIDE(_stack_bottom = __stack_bottom);

/* Sanity checks (facultatifs mais sages) */
ASSERT((KERNEL_VMA & (PAGE_SIZE-1)) == 0, "KERNEL_VMA doit être aligné page.");
ASSERT((KERNEL_LMA & (PAGE_SIZE-1)) == 0, "KERNEL_LMA doit être aligné page.");
