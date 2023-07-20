
//! we are able to include some standard library features like types, given 
//! that the standard library feature is not in 
//! anyway related to the underlying os

#include <stdint.h>

#define VGA_BUF_ADR 0xb8000

//! yes this puts something randomly to the vga buffer :) 
void c_test()
{
    uint8_t* vga_bug = (uint8_t*)VGA_BUF_ADR;

    const char* h = "This is a C test"; 

    for(int i = 0; h[i] != '\0'; i++)
    {
        vga_bug[i * 2] = h[i];
        vga_bug[i * 2 + 1] = 0x0B;  
    }
}

