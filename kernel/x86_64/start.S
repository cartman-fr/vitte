/* x86_64/start.S — bootstrap kernel
 * - Boot: Multiboot2 (32-bit protected mode)
 * - Switch to Long Mode (64-bit), identity map low 1GiB (2MiB pages)
 * - Load minimal GDT, set stack, clear .bss, call kmain()
 * - Provide ISR stubs: isr_default, isr_timer, isr_keyboard, isr_de, isr_gp, isr_pf
 */

    .intel_syntax noprefix
    .section .multiboot2, "a"
    .align 8
/* Multiboot2 header (minimum): magic + arch + length + checksum + end tag */
mb2_header:
    .long 0xE85250D6          /* magic */
    .long 0                    /* architecture (i386) */
    .long mb2_end - mb2_header /* header length */
    .long -(0xE85250D6 + 0 + (mb2_end - mb2_header)) /* checksum */

    /* End tag */
    .short 0
    .short 0
    .long 8
mb2_end:

/* Symbols fournis par le linker */
    .extern __bss_start
    .extern __bss_end
    .extern __stack_top
    .extern kmain

/* GDT minimal: null, code64, data */
    .section .data
    .align 8
gdt64:
    .quad 0x0000000000000000          /* null */
    .quad 0x00AF9A000000FFFF          /* code 64 */
    .quad 0x00AF92000000FFFF          /* data 64 */
gdt64_ptr:
    .word (3*8 - 1)
    .quad gdt64

/* Tables de pages (4K align) */
    .section .bss
    .align 4096
pml4:
    .zero 4096
pdpt:
    .zero 4096
pde0:
    .zero 4096

/* Code de bootstrap (32-bit), point d’entrée _start */
    .section .text.boot, "ax"
    .globl _start
_start:
    /* Assumé: protected mode 32-bit, paging off, A20 on, interrupts off */

    /* Charger GDT 64 (compatible legacy) */
    lgdt [gdt64_ptr]

    /* Préparer tables de pages:
       PML4[0] -> PDPT | P=1,RW=1
       PDPT[0] -> PDE0 | P=1,RW=1
       PDE0[i] mappe i*2MiB avec PS=1 */
    lea eax, [pdpt]
    or eax, 0b11
    mov dword ptr [pml4 + 0*8 + 0], eax
    mov dword ptr [pml4 + 0*8 + 4], 0

    lea eax, [pde0]
    or eax, 0b11
    mov dword ptr [pdpt + 0*8 + 0], eax
    mov dword ptr [pdpt + 0*8 + 4], 0

    /* Remplir PDE0 (512 entrées * 2MiB = 1GiB) */
    xor ecx, ecx                 /* i = 0 */
    mov edx, 0                   /* base = 0 */
.fill_pde:
    /* Entry = base | P=1 | RW=1 | PS=1 (bit 7) */
    mov eax, edx
    or eax, (1 << 0) | (1 << 1) | (1 << 7)
    mov dword ptr [pde0 + ecx*8 + 0], eax
    mov dword ptr [pde0 + ecx*8 + 4], 0
    add edx, 0x200000            /* +2MiB */
    inc ecx
    cmp ecx, 512
    jne .fill_pde

    /* CR3 = PML4 base */
    lea eax, [pml4]
    mov cr3, eax

    /* Activer PAE (CR4.PAE=1) */
    mov eax, cr4
    or eax, (1 << 5)
    mov cr4, eax

    /* EFER.LME=1 (MSR 0xC000_0080) */
    mov ecx, 0xC0000080
    rdmsr
    or eax, (1 << 8)
    wrmsr

    /* Paging ON (CR0.PG=1) + PE déjà à 1 */
    mov eax, cr0
    or eax, (1 << 31)
    mov cr0, eax

    /* Far jump vers 64-bit code segment */
    push dword 0x08              /* CS selector (code 64) bas sur 32-bit push */
    push dword offset .longmode
    retf

/* ======================= LONG MODE 64-bit ======================= */
    .code64
.longmode:
    /* Recharger segments data (0x10) */
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax

    /* Installer la pile */
    lea rsp, [__stack_top]

    /* Nettoyer .bss */
    lea rdi, [__bss_start]
    lea rcx, [__bss_end]
    sub rcx, rdi
    xor eax, eax
    rep stosb

    /* SAUT dans kmain() */
    call kmain

.hang:
    cli
    hlt
    jmp .hang

/* ======================= ISRs 64-bit ======================= */
/* Stubs simples :
   - Sauvent quelques registres volatiles,
   - Appellent les callbacks Vitte,
   - Gèrent l’éventuel code d’erreur (#GP, #PF),
   - iretq.
*/
    .extern irq_timer_cb
    .extern irq_kbd_cb
    .extern isr_default_cb
    .extern isr_gp_cb
    .extern isr_pf_cb
    .extern isr_de_cb

/* Macro save/restore (rapide) */
.macro PUSH_VOLATILES
    push rax
    push rcx
    push rdx
    push rsi
    push rdi
    push r8
    push r9
    push r10
    push r11
.endm

.macro POP_VOLATILES
    pop r11
    pop r10
    pop r9
    pop r8
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rax
.endm

    .text
    .align 16
    .globl isr_default
isr_default:
    PUSH_VOLATILES
    call isr_default_cb
    POP_VOLATILES
    iretq

    .globl isr_timer       /* IRQ0 remappée -> vecteur 32 */
isr_timer:
    PUSH_VOLATILES
    call irq_timer_cb
    POP_VOLATILES
    iretq

    .globl isr_keyboard    /* IRQ1 remappée -> vecteur 33 */
isr_keyboard:
    PUSH_VOLATILES
    call irq_kbd_cb
    POP_VOLATILES
    iretq

    .globl isr_de          /* #DE Divide Error (sans error code) */
isr_de:
    PUSH_VOLATILES
    call isr_de_cb
    POP_VOLATILES
    iretq

    .globl isr_gp          /* #GP General Protection (AVEC error code) */
isr_gp:
    PUSH_VOLATILES
    call isr_gp_cb
    POP_VOLATILES
    add rsp, 8             /* drop error code */
    iretq

    .globl isr_pf          /* #PF Page Fault (AVEC error code) */
isr_pf:
    PUSH_VOLATILES
    call isr_pf_cb
    POP_VOLATILES
    add rsp, 8             /* drop error code */
    iretq
