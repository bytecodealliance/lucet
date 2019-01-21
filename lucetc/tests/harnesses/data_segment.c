#include "vm.h"
#include <assert.h>
#include <malloc.h>
#include <stdint.h>
#include <string.h>

// IMPORTANT: this is a copy of the definition of this struct in liblucet.
// If changes are made to this, make sure they are also made in liblucet!
struct wasm_data_segment {
    uint32_t memory_index;
    uint32_t offset;
    uint32_t length;
    char     data[];
};

void guest_func_main(struct vmctx *);

extern char wasm_data_segments[];
extern int  wasm_data_segments_len;

int main()
{
    // Run guest program and ensure the result is correct (see corresponding .wat
    // for details)
    struct VM *vm = make_vm();
    guest_func_main(get_vmctx(vm));

    uint32_t output          = ((uint32_t *) vm->heap)[0];
    uint32_t expected_output = 0;
    if (output != expected_output) {
        printf("Output was %u\n", output);
        return 1;
    }

    // Define expected data segment initialization data

    int                       n_expected = 3;
    struct wasm_data_segment *expected[n_expected];

    const char                bytes_0[] = { '9', '9', '9', '9', '9' };
    struct wasm_data_segment *ds = malloc(sizeof(struct wasm_data_segment) + sizeof(bytes_0));
    memcpy(ds,
           &(struct wasm_data_segment const){
               .memory_index = 0, .offset = 0, .length = sizeof(bytes_0) },
           sizeof(struct wasm_data_segment));
    memcpy((char *) ds + sizeof(struct wasm_data_segment), bytes_0, sizeof(bytes_0));
    expected[0] = ds;

    const char bytes_1[] = { 0xaa, 0xbb }; // see .wat for defn
    ds                   = malloc(sizeof(struct wasm_data_segment) + sizeof(bytes_1));
    memcpy(ds,
           &(struct wasm_data_segment const){
               .memory_index = 0, .offset = 0, .length = sizeof(bytes_1) },
           sizeof(struct wasm_data_segment));
    memcpy((char *) ds + sizeof(struct wasm_data_segment), bytes_1, sizeof(bytes_1));
    expected[1] = ds;

    const char bytes_2[] = { 0xcc, 0xdd }; // see .wat for defn
    ds                   = malloc(sizeof(struct wasm_data_segment) + sizeof(bytes_2));
    memcpy(ds,
           &(struct wasm_data_segment const){
               .memory_index = 0, .offset = 1, .length = sizeof(bytes_2) },
           sizeof(struct wasm_data_segment));
    memcpy((char *) ds + sizeof(struct wasm_data_segment), bytes_2, sizeof(bytes_2));
    expected[2] = ds;

    // Make sure data segment initialization data in ELF matches expectation

    int i = 0; // current data segment
    int p = 0; // current position in wasm_data_segment
    while (p < wasm_data_segments_len) {
        struct wasm_data_segment *deserialized =
            (struct wasm_data_segment *) &wasm_data_segments[p];
        assert(deserialized->length == expected[i]->length);
        assert(deserialized->memory_index == expected[i]->memory_index);
        assert(deserialized->offset == expected[i]->offset);

        int j;
        for (j = 0; j < deserialized->length; j++) {
            assert(deserialized->data[j] == expected[i]->data[j]);
        }
        p += sizeof(struct wasm_data_segment) + deserialized->length;
        p += (8 - p % 8) % 8;
        i += 1;
    }

    for (i = 0; i < n_expected; i++) {
        free(expected[i]);
    }

    return 0;
}
