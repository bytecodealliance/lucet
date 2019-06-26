#include <stdio.h>

extern void func_a(int);

void func_b(int b) {
    for (int i = 0; i < b; i++) {
        printf("func_b! %d of %d\n", i, b);
        func_a(i);
    }
}
