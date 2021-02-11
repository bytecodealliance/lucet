#include <sys/stat.h>
#include <sys/time.h>

#include <stdlib.h>
#include <assert.h>
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <sched.h>
#include <stdio.h>
#include <time.h>
#include <unistd.h>
#include <utime.h>

int main(void)
{
    struct timespec times[2];
    struct dirent * entry;
    char            buf[4];
    DIR *           dir;
    FILE *          fp;
    struct stat     st;
    off_t           offset;
    int             fd;
    int             res;

    fd = open("/sandbox/testfile", O_CREAT | O_RDWR, 0644);
    if (fd == -1) {
        exit(1);
    }

    res = posix_fallocate(fd, 0, 10000);
    if (res != 0) {
        exit(2);
    }

    res = fstat(fd, &st);
    if (res != 0) {
        exit(3);
    }
    if (st.st_size != 10000) {
        exit(4);
    }

    res = ftruncate(fd, 1000);
    if (res != 0) {
        exit(5);
    }
    res = fstat(fd, &st);
    if (res != 0) {
        exit(6);
    }
    if (st.st_size != 1000) {
        exit(7);
    }
    if (st.st_nlink != 1) {
        exit(8);
    }
    res = posix_fadvise(fd, 0, 1000, POSIX_FADV_RANDOM);
    if (res != 0) {
        exit(9);
    }

    res = (int) write(fd, "test", 4);
    if (res != 4) {
        exit(10);
    }

    offset = lseek(fd, 0, SEEK_CUR);
    if (offset != 4) {
        exit(11);
    }
    offset = lseek(fd, 0, SEEK_END);
    if (offset != 1000) {
        exit(12);
    }
    offset = lseek(fd, 0, SEEK_SET);
    if (offset != 0) {
        exit(13);
    }

    res = fdatasync(fd);
    if (res != 0) {
        exit(14);
    }

    res = fsync(fd);
    if (res != 0) {
        exit(15);
    }

    times[0] = (struct timespec){ .tv_sec = 1557403800, .tv_nsec = 0 };
    times[1] = (struct timespec){ .tv_sec = 1557403800, .tv_nsec = 0 };

    res = futimens(fd, times);
    if (res != 0) {
        exit(16);
    }

    res = pread(fd, buf, sizeof buf, 2);
    if (res != 4) {
        exit(17);
    }
    if (buf[1] != 't') {
        exit(18);
    }

    res = pwrite(fd, "T", 1, 3);
    if (res != 1) {
        exit(19);
    }

    res = pread(fd, buf, sizeof buf, 2);
    if (res != 4) {
        exit(20);
    }
    if (buf[1] != 'T') {
        exit(21);
    }

    res = close(fd);
    if (res != 0) {
        exit(22);
    }

    dir = opendir("/nonexistent");
    if (dir != NULL) {
        exit(23);
    }

    res = mkdir("/sandbox/test", 0755);
    if (res != 0) {
        exit(24);
    }

    res = mkdir("/sandbox/test", 0755);
    if (res != -1) {
        exit(25);
    }
    if (errno != EEXIST) {
        exit(26);
    }

    res = rmdir("/sandbox/test");
    if (res != 0) {
        exit(27);
    }

    res = rmdir("/sandbox/test");
    if (res != -1) {
        exit(28);
    }

    res = rename("/sandbox/testfile", "/sandbox/testfile2");
    if (res != 0) {
        exit(29);
    }

    res = unlink("/sandbox/testfile");
    if (res != -1) {
        exit(30);
    }

    res = access("/sandbox/testfile2", R_OK);
    if (res != 0) {
        exit(31);
    }

    res = link("/sandbox/testfile2", "/sandbox/testfile-link");
    if (res != 0) {
        exit(32);
    }

    res = access("/sandbox/testfile-link", R_OK);
    if (res != 0) {
        exit(33);
    }

    res = symlink("./testfile-link", "/sandbox/testfile-symlink");
    if (res != 0) {
        exit(34);
    }

    res = symlink("./testfile2", "/sandbox/testfile-symlink");
    if (res != -1) {
        exit(35);
    }

    res = sched_yield();
    if (res != 0) {
        exit(36);
    }

    fd = open("/sandbox/testfile2", O_RDONLY);
    if (fd == -1) {
        exit(37);
    }

    fp = fdopen(fd, "r");
    if (fp == NULL) {
        exit(38);
    }

    res = fgetc(fp);
    if (res != 't') {
        exit(39);
    }

    res = fclose(fp);
    if (res != 0) {
        exit(40);
    }

    dir = opendir("/sandbox");
    if (dir == NULL) {
        exit(41);
    }

    res = 0;
    while ((entry = readdir(dir)) != NULL) {
        if (entry->d_name[0] == '.') {
            res += 1000;
        } else {
            res++;
        }
    }
    if (res != 2003) {
        exit(42);
    }

    res = closedir(dir);
    if (res != 0) {
        exit(43);
    }

    res = mkdir("/sandbox/a", 0755);
    if (res != 0) {
        exit(44);
    }
    res = mkdir("/sandbox/a/b", 0755);
    if (res != 0) {
        exit(45);
    }
    res = mkdir("/sandbox/a/b/c", 0755);
    if (res != 0) {
        exit(46);
    }
    res = access("/sandbox/a/b/c", R_OK);
    if (res != 0) {
        exit(47);
    }

    return 0;
}
