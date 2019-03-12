#include "guest_module.h"
#include <stdio.h>
#include <stdlib.h>

const char *guest_module_path(const char *name)
{
    const char *prefix = getenv("GUEST_MODULE_PREFIX");
    if (prefix) {
        char *relroot = "lucet-runtime-c/test/build";
        char *path = malloc(strlen(name) + strlen(relroot) + strlen(prefix) + 3);
        sprintf(path, "%s/%s/%s", prefix, relroot, name);
        return path;
    } else {
        const char *prefix = "build";
        char *      path   = malloc(strlen(name) + strlen(prefix) + 2);
        sprintf(path, "%s/%s", prefix, name);
        return path;
    }
}
