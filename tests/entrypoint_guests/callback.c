#include <stdint.h>

extern uint64_t callback_hostcall(uint64_t (*)(uint64_t), uint64_t);

uint64_t callback_callback(uint64_t x)
{
    return x + 1;
}

__attribute__((visibility("default"))) uint64_t callback_entrypoint(uint64_t x)
{
    return callback_hostcall(callback_callback, x) + 1;
}
