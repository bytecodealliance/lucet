#include <stdio.h>

void func_a(int a) {
    for (int i = 0; i < a; i++) {
        printf("func_a! %d of %d\n", i, a);
    }
}
