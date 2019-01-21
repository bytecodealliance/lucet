char hello[] = "hello";

__attribute__((visibility("default"))) int main(void)
{
    // Intentional out-of-bounds access
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Warray-bounds"
    ((char *) hello)[(1024 * 1024) + 1] = '\n';
#pragma clang diagnostic pop
    return 0;
}
