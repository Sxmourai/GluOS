; TO COMPILE: nasm -felf64 kernel.asm -o kernel.o
_start:
    mov byte [0xB8000], 'g'
    jmp $
 
 