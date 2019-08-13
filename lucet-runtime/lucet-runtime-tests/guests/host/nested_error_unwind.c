#include <stddef.h>
#include <stdint.h>

extern uint64_t nested_error_unwind_outer(uint64_t (*)(void));
extern void     nested_error_unwind_inner();

uint64_t callback(void)
{
    nested_error_unwind_inner();
    return 0;
}

uint64_t entrypoint(void)
{
    return nested_error_unwind_outer(callback);
}

extern uint64_t nested_error_unwind_regs_outer(uint64_t (*)(void));
extern void     nested_error_unwind_regs_inner();

uint64_t callback_regs(void)
{
    uint64_t a = 0xFFFFFFFF00000000;
    uint64_t b = 0xFFFFFFFF00000001;
    uint64_t c = 0xFFFFFFFF00000002;
    uint64_t d = 0xFFFFFFFF00000003;
    uint64_t e = 0xFFFFFFFF00000004;
    uint64_t f = 0xFFFFFFFF00000005;
    uint64_t g = 0xFFFFFFFF00000006;
    uint64_t h = 0xFFFFFFFF00000007;
    uint64_t i = 0xFFFFFFFF00000008;
    uint64_t j = 0xFFFFFFFF00000009;
    uint64_t k = 0xFFFFFFFF0000000A;
    uint64_t l = 0xFFFFFFFF0000000B;

    a = b + c ^ 0;
    b = c + d ^ 1;
    c = d + e ^ 2;
    d = e + f ^ 3;
    e = f + g ^ 4;
    f = g + h ^ 5;
    g = h + i ^ 6;
    h = i + j ^ 7;
    i = j + k ^ 8;
    j = k + l ^ 9;
    k = l + a ^ 10;
    l = a + b ^ 11;

    nested_error_unwind_regs_inner();

    a = b * c & 0;
    b = c * d & 1;
    c = d * e & 2;
    d = e * f & 3;
    e = f * g & 4;
    f = g * h & 5;
    g = h * i & 6;
    h = i * j & 7;
    i = j * k & 8;
    j = k * l & 9;
    k = l * a & 10;
    l = a * b & 11;
    return l;
}

uint64_t entrypoint_regs(void)
{
    return nested_error_unwind_regs_outer(callback_regs);
}
