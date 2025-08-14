/* kernel/armv7em/start.S
 * Startup & vector table — ARMv7-M/EM (Cortex-M4/M7)
 * GNU as / GCC bare-metal friendly.
 *
 * Attentes:
 *  - _estack  : défini par linker.ld (pointe sur le haut de la RAM)
 *  - Reset_Handler : implémenté dans kmain.vitte (no_std)
 *
 * Remarques:
 *  - On NE fait pas ici la copie .data / zéro .bss : tu le fais déjà en Vitte.
 *  - Si tu veux déplacer la VTOR à l’exécution, fais-le dans Reset_Handler.
 */

    .syntax unified
    .thumb
    .cpu cortex-m4
    .fpu fpv4-sp-d16
    .thumb

    .extern Reset_Handler
    .extern _estack

    /* ———————————————————————————————————————
     * Table des vecteurs (au tout début de FLASH)
     * ——————————————————————————————————————— */
    .section .isr_vector, "a", %progbits
    .align  8          /* VTOR exige un alignement (souvent 128/256). Le linker assure le VECT_ALIGN. */
    .global __isr_vector
__isr_vector:
    .word   _estack                /* 0: SP initial            */
    .word   Reset_Handler          /* 1: Reset                 */
    .word   NMI_Handler            /* 2: NMI                   */
    .word   HardFault_Handler      /* 3: HardFault             */
    .word   MemManage_Handler      /* 4: MemManage             */
    .word   BusFault_Handler       /* 5: BusFault              */
    .word   UsageFault_Handler     /* 6: UsageFault            */
    .word   0                      /* 7: Reserved              */
    .word   0                      /* 8: Reserved              */
    .word   0                      /* 9: Reserved              */
    .word   0                      /* 10: Reserved             */
    .word   SVC_Handler            /* 11: SVCall               */
    .word   DebugMon_Handler       /* 12: DebugMonitor         */
    .word   0                      /* 13: Reserved             */
    .word   PendSV_Handler         /* 14: PendSV               */
    .word   SysTick_Handler        /* 15: SysTick              */

    /* IRQ 0..63 (placeholder, à surcharger si besoin) */
    .word   IRQ0_Handler
    .word   IRQ1_Handler
    .word   IRQ2_Handler
    .word   IRQ3_Handler
    .word   IRQ4_Handler
    .word   IRQ5_Handler
    .word   IRQ6_Handler
    .word   IRQ7_Handler
    .word   IRQ8_Handler
    .word   IRQ9_Handler
    .word   IRQ10_Handler
    .word   IRQ11_Handler
    .word   IRQ12_Handler
    .word   IRQ13_Handler
    .word   IRQ14_Handler
    .word   IRQ15_Handler
    .word   IRQ16_Handler
    .word   IRQ17_Handler
    .word   IRQ18_Handler
    .word   IRQ19_Handler
    .word   IRQ20_Handler
    .word   IRQ21_Handler
    .word   IRQ22_Handler
    .word   IRQ23_Handler
    .word   IRQ24_Handler
    .word   IRQ25_Handler
    .word   IRQ26_Handler
    .word   IRQ27_Handler
    .word   IRQ28_Handler
    .word   IRQ29_Handler
    .word   IRQ30_Handler
    .word   IRQ31_Handler
   .word   IRQ32_Handler
    .word   IRQ33_Handler
    .word   IRQ34_Handler
    .word   IRQ35_Handler
    .word   IRQ36_Handler
    .word   IRQ37_Handler
    .word   IRQ38_Handler
    .word   IRQ39_Handler
    .word   IRQ40_Handler
    .word   IRQ41_Handler
    .word   IRQ42_Handler
    .word   IRQ43_Handler
    .word   IRQ44_Handler
    .word   IRQ45_Handler
    .word   IRQ46_Handler
    .word   IRQ47_Handler
    .word   IRQ48_Handler
    .word   IRQ49_Handler
    .word   IRQ50_Handler
    .word   IRQ51_Handler
    .word   IRQ52_Handler
    .word   IRQ53_Handler
    .word   IRQ54_Handler
    .word   IRQ55_Handler
    .word   IRQ56_Handler
    .word   IRQ57_Handler
    .word   IRQ58_Handler
    .word   IRQ59_Handler
    .word   IRQ60_Handler
    .word   IRQ61_Handler
    .word   IRQ62_Handler
    .word   IRQ63_Handler

    /* ———————————————————————————————————————
     * Handlers par défaut (weak) → boucle infinie
     * On aliase tous les handlers vers Default_Handler,
     * sauf si tu fournis une impl concrète ailleurs.
     * ——————————————————————————————————————— */
    .text
    .thumb
    .align  2

    .global Default_Handler
    .thumb_func
Default_Handler:
1:  b   1b

    /* Aliases faibles */
    .weak   NMI_Handler
    .weak   HardFault_Handler
    .weak   MemManage_Handler
    .weak   BusFault_Handler
    .weak   UsageFault_Handler
    .weak   SVC_Handler
    .weak   DebugMon_Handler
    .weak   PendSV_Handler
    .weak   SysTick_Handler

    .weak   IRQ0_Handler
    .weak   IRQ1_Handler
    .weak   IRQ2_Handler
    .weak   IRQ3_Handler
    .weak   IRQ4_Handler
    .weak   IRQ5_Handler
    .weak   IRQ6_Handler
    .weak   IRQ7_Handler
    .weak   IRQ8_Handler
    .weak   IRQ9_Handler
    .weak   IRQ10_Handler
    .weak   IRQ11_Handler
    .weak   IRQ12_Handler
    .weak   IRQ13_Handler
    .weak   IRQ14_Handler
    .weak   IRQ15_Handler
    .weak   IRQ16_Handler
    .weak   IRQ17_Handler
    .weak   IRQ18_Handler
    .weak   IRQ19_Handler
    .weak   IRQ20_Handler
    .weak   IRQ21_Handler
    .weak   IRQ22_Handler
    .weak   IRQ23_Handler
    .weak   IRQ24_Handler
    .weak   IRQ25_Handler
    .weak   IRQ26_Handler
    .weak   IRQ27_Handler
    .weak   IRQ28_Handler
    .weak   IRQ29_Handler
    .weak   IRQ30_Handler
    .weak   IRQ31_Handler
    .weak   IRQ32_Handler
    .weak   IRQ33_Handler
    .weak   IRQ34_Handler
    .weak   IRQ35_Handler
    .weak   IRQ36_Handler
    .weak   IRQ37_Handler
    .weak   IRQ38_Handler
    .weak   IRQ39_Handler
    .weak   IRQ40_Handler
    .weak   IRQ41_Handler
    .weak   IRQ42_Handler
    .weak   IRQ43_Handler
    .weak   IRQ44_Handler
    .weak   IRQ45_Handler
    .weak   IRQ46_Handler
    .weak   IRQ47_Handler
    .weak   IRQ48_Handler
    .weak   IRQ49_Handler
    .weak   IRQ50_Handler
    .weak   IRQ51_Handler
    .weak   IRQ52_Handler
    .weak   IRQ53_Handler
    .weak   IRQ54_Handler
    .weak   IRQ55_Handler
    .weak   IRQ56_Handler
    .weak   IRQ57_Handler
    .weak   IRQ58_Handler
    .weak   IRQ59_Handler
    .weak   IRQ60_Handler
    .weak   IRQ61_Handler
    .weak   IRQ62_Handler
    .weak   IRQ63_Handler

    /* Redirections faibles vers Default_Handler
       (GNU as: .thumb_set crée un alias au link) */
    .thumb_set NMI_Handler,         Default_Handler
    .thumb_set HardFault_Handler,   Default_Handler
    .thumb_set MemManage_Handler,   Default_Handler
    .thumb_set BusFault_Handler,    Default_Handler
    .thumb_set UsageFault_Handler,  Default_Handler
    .thumb_set SVC_Handler,         Default_Handler
    .thumb_set DebugMon_Handler,    Default_Handler
    .thumb_set PendSV_Handler,      Default_Handler
    .thumb_set SysTick_Handler,     Default_Handler

    .thumb_set IRQ0_Handler,  Default_Handler
    .thumb_set IRQ1_Handler,  Default_Handler
    .thumb_set IRQ2_Handler,  Default_Handler
    .thumb_set IRQ3_Handler,  Default_Handler
    .thumb_set IRQ4_Handler,  Default_Handler
    .thumb_set IRQ5_Handler,  Default_Handler
    .thumb_set IRQ6_Handler,  Default_Handler
    .thumb_set IRQ7_Handler,  Default_Handler
    .thumb_set IRQ8_Handler,  Default_Handler
    .thumb_set IRQ9_Handler,  Default_Handler
    .thumb_set IRQ10_Handler, Default_Handler
    .thumb_set IRQ11_Handler, Default_Handler
    .thumb_set IRQ12_Handler, Default_Handler
    .thumb_set IRQ13_Handler, Default_Handler
    .thumb_set IRQ14_Handler, Default_Handler
    .thumb_set IRQ15_Handler, Default_Handler
    .thumb_set IRQ16_Handler, Default_Handler
    .thumb_set IRQ17_Handler, Default_Handler
    .thumb_set IRQ18_Handler, Default_Handler
    .thumb_set IRQ19_Handler, Default_Handler
    .thumb_set IRQ20_Handler, Default_Handler
    .thumb_set IRQ21_Handler, Default_Handler
    .thumb_set IRQ22_Handler, Default_Handler
    .thumb_set IRQ23_Handler, Default_Handler
    .thumb_set IRQ24_Handler, Default_Handler
    .thumb_set IRQ25_Handler, Default_Handler
    .thumb_set IRQ26_Handler, Default_Handler
    .thumb_set IRQ27_Handler, Default_Handler
    .thumb_set IRQ28_Handler, Default_Handler
    .thumb_set IRQ29_Handler, Default_Handler
    .thumb_set IRQ30_Handler, Default_Handler
    .thumb_set IRQ31_Handler, Default_Handler
    .thumb_set IRQ32_Handler, Default_Handler
    .thumb_set IRQ33_Handler, Default_Handler
    .thumb_set IRQ34_Handler, Default_Handler
    .thumb_set IRQ35_Handler, Default_Handler
    .thumb_set IRQ36_Handler, Default_Handler
    .thumb_set IRQ37_Handler, Default_Handler
    .thumb_set IRQ38_Handler, Default_Handler
    .thumb_set IRQ39_Handler, Default_Handler
    .thumb_set IRQ40_Handler, Default_Handler
    .thumb_set IRQ41_Handler, Default_Handler
    .thumb_set IRQ42_Handler, Default_Handler
    .thumb_set IRQ43_Handler, Default_Handler
    .thumb_set IRQ44_Handler, Default_Handler
    .thumb_set IRQ45_Handler, Default_Handler
    .thumb_set IRQ46_Handler, Default_Handler
    .thumb_set IRQ47_Handler, Default_Handler
    .thumb_set IRQ48_Handler, Default_Handler
    .thumb_set IRQ49_Handler, Default_Handler
    .thumb_set IRQ50_Handler, Default_Handler
    .thumb_set IRQ51_Handler, Default_Handler
    .thumb_set IRQ52_Handler, Default_Handler
    .thumb_set IRQ53_Handler, Default_Handler
    .thumb_set IRQ54_Handler, Default_Handler
    .thumb_set IRQ55_Handler, Default_Handler
    .thumb_set IRQ56_Handler, Default_Handler
    .thumb_set IRQ57_Handler, Default_Handler
    .thumb_set IRQ58_Handler, Default_Handler
    .thumb_set IRQ59_Handler, Default_Handler
    .thumb_set IRQ60_Handler, Default_Handler
    .thumb_set IRQ61_Handler, Default_Handler
    .thumb_set IRQ62_Handler, Default_Handler
    .thumb_set IRQ63_Handler, Default_Handler
