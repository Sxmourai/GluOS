BITS 16
    cli
    cld
    jmp far, 0x8000:0x40
    ALIGN 16
_L8010_GDT_table:
    dl 0, 0
    dl 0x0000FFFF, 0x00CF9A00    ; flat code
    dl 0x0000FFFF, 0x008F9200    ; flat data
    dl 0x00000068, 0x00CF8900    ; tss
_L8030_GDT_value:
    dw _L8030_GDT_value - _L8010_GDT_table - 1
    dl 0x8010
    dl 0, 0
    ALIGN 64
_L8040_32:
    xorw    ax, ax
    movw    ax, ds
    lgdtl   0x8030
    movl    cr0, eax
    orl     1, eax
    movl    eax, cr0
    ljmp    8, 0x8060
    .align 32
    BITS 32
_L8060_main:
    mov al, 'g'
    mov [0xb8000], al
    mov [0xb8001], al
    mov [0xb8002], al
    ; TODO: Jump to Rust
spin:
    jmp spin
; So that the compiler is happy, but unreachable