#include <stdint.h>
#include "riscYstdio.h"
    
int main() {
    printf("Hello World from Emulator\r\n");
    int test = 0;
    while(1){
        for(volatile int i=0;i<100000;i++); // delay
        printfSCR(SCREEN_WIDTH*2,0xf,"Count: %d\r\n", test++);
    }
}