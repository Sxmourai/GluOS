; this code will be relocated to 0x8000, sets up environment for calling a C function
    .code16
ap_trampoline:
    cli
    cld
    ljmp    $0, $0x8040
    .align 16
_L8010_GDT_table:
    .long 0, 0
    .long 0x0000FFFF, 0x00CF9A00    ; flat code
    .long 0x0000FFFF, 0x008F9200    ; flat data
    .long 0x00000068, 0x00CF8900    ; tss
_L8030_GDT_value:
    .word _L8030_GDT_value - _L8010_GDT_table - 1
    .long 0x8010
    .long 0, 0
    .align 64
_L8040:
    xorw    %ax, %ax
    movw    %ax, %ds
    lgdtl   0x8030
    movl    %cr0, %eax
    orl     $1, %eax
    movl    %eax, %cr0
    ljmp    $8, $0x8060
    .align 32
    .code32
_L8060:
    movw    $16, %ax
    movw    %ax, %ds
    movw    %ax, %ss
    ; get our Local APIC ID
    mov     $1, %eax
    cpuid
    shrl    $24, %ebx
    movl    %ebx, %edi
    ; set up 32k stack, one for each core. It is important that all core must have its own stack
    shll    $15, %ebx
    movl    stack_top, %esp
    subl    %ebx, %esp
    pushl   %edi
    ; spinlock, wait for the BSP to finish
1:  pause
    cmpb    $0, bspdone
    jz      1b
    lock    incb aprunning
    ; jump into C code (should never return)
    ljmp    $8, $ap_startup