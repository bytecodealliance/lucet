#include <sys/types.h>
#include <sys/stat.h>

#include <stdlib.h>
#include <stdio.h>
#include <dirent.h>
#include <errno.h>

int main(void)
{

    int res = mkdir("/sandbox/a", 0755);
    if (res != 0) {
        exit(1);
    }
    res = mkdir("/sandbox/b", 0755);
    if (res != 0) {
        exit(2);
    }
    res = mkdir("/sandbox/c", 0755);
    if (res != 0) {
        exit(3);
    }

    DIR* dir = opendir("/sandbox");
    if (dir == NULL) {
        printf("opendir failed: %d\n", errno);
        exit(4);
    }

    res = 0;
    struct dirent * entry = NULL;
    while ((entry = readdir(dir)) != NULL) {
        if (entry->d_name[0] == '.') {
            res += 1000;
        } else {
            res++;
        }
    }
    if (res != 2003) {
        printf("readdir result: %d, errno: %d\n", res, errno);
        exit(5);
    }

    res = closedir(dir);
    if (res != 0) {
        exit(6);
    }

    exit(0);
}

