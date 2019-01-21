#include <stdint.h>
#include <stdio.h>

#include "../src/lucet_context_private.h"

struct lucet_context parent_regs;
struct lucet_context child_regs;

void returning_child(void *arg0, void *arg1)
{
    printf("child!\n");
}

#define STACK_WORDS 64
uint64_t stack[STACK_WORDS] = { 0 };

int main(void)
{
    char *stack_top = (char *) &stack[STACK_WORDS - 1];

    lucet_context_init(&child_regs, returning_child, stack, stack_top, stack_top, &parent_regs);

    printf("parent!\n");

    lucet_context_swap(&parent_regs, &child_regs);

    printf("parent again!\n");

    return 0;
}
