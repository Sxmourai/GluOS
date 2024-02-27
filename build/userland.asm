section .text
   global _start
_start:
    mov byte [0xB8000], 'g'
    jmp $
; To compile: nasm -felf64 userland.asm && ld userland.o -o userland
 