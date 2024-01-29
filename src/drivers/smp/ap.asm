; https://github.com/daniel-keitel/Steelmind_OS/blob/main/kernel/smp_trampoline/ap.asm
; This file is not part of the build system unless run with -a.
; The coresponding binary is included in the repository to avoid a build dependency on nasm.
; to assemble this file run: nasm -fbin ap.asm -o ap.bin or use the build system with -a

bits 16

%define L4_TABLE_ADDR       0x0A00
%define CORE_COUNTER_ADDR   0x0B00
%define STACK_STRIDE        0x0B10
%define STACK_BASE_ADDR     0x0B20
%define ENTRY_FUNCTION_ADDR 0x0B30


real_mode:
    ; disable interrupts
    cli
    cld

    ; set up segment registers
    mov ax, 0
    mov ss, ax
    mov ds, ax
    mov es, ax

    ; set cr3 to same l4 page table as the bsp
    mov ecx, [L4_TABLE_ADDR]
    mov cr3, ecx

    ; prepare for long mode by enabling physical address extensions (bit 5)
    mov eax, cr4
    or  eax, 1 << 5             
    mov cr4, eax

    ; modify EFER MSR to enable long mode (bit 8) and no execute enable (bit 11)
    mov ecx, 0xC0000080
    rdmsr
    or  eax, 1 << 8
    or  eax, 1 << 11 ; ? 
    wrmsr

    ; set temporary global descriptor table
    lgdt [GDT.Pointer]

    ; enable paging and protection; triggering the switch to long mode
    mov eax, cr0
    or  eax, 0x80000001
    mov cr0, eax

    ; jump into 64 bit assembly (needs to happend imidiatly after triggering long mode)
    jmp dword 0x08:long_mode

GDT:
.Null:
    dq 0x0000000000000000            
 
.Code:
    dq 0x00209A0000000000             
    dq 0x0000920000000000            
 
ALIGN 4
    dw 0                              
 
.Pointer:
    dw $ - GDT - 1                    
    dd GDT        

bits 64

long_mode:
    ; set up segment registers
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; dont emulate coprocessor (only monitor)
    mov rax, cr0
    and rax, ~(1 << 2)                  
    or  rax, 1 << 1                     
    mov cr0, rax

setup:
    ; get address of atomic core counter
    mov rax, qword [CORE_COUNTER_ADDR] 
    ; increment atomic core counter and store ap index in rbp
    mov rbp, 1 
    lock xadd [rax], rbp

    ; pass index to entry function 
    mov rdi, rbp

    ; multiply index by stack stride to get stack offset
    imul rbp, [STACK_STRIDE]
    ; add base address and offset
    add rbp, [STACK_BASE_ADDR]

    ; set stack pointer
    mov rsp, rbp

    ; call entry function with start index as argument (the function never returns)
    jmp [ENTRY_FUNCTION_ADDR]

    ; safety loop (should never be reached)
    jmp $