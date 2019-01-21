__attribute__((visibility("default"))) int trigger_div_error(int i)
{
    int z = 100 / i;
    return z;
}
