#include <stddef.h>
#include <stdint.h>
#include <assert.h>

extern void hostcall_containing_block_on(int);
extern void hostcall_block_on_access_vmctx(uint8_t*, uint8_t);

static uint8_t some_byte = 0;

int main(void)
{
    hostcall_containing_block_on(1312);
    hostcall_block_on_access_vmctx(&some_byte, 1);
    if (some_byte == 1) {
        return 0;
    } else {
        return 1;
    }
}
