#include <assert.h>
#include <poll.h>
#include <stdio.h>
#include <time.h>
#include <unistd.h>

int main(void)
{
    struct pollfd fds[2];
    time_t        before, now;
    int           ret;

    fds[0] = (struct pollfd){ .fd = 1, .events = POLLOUT, .revents = 0 };
    fds[1] = (struct pollfd){ .fd = 2, .events = POLLOUT, .revents = 0 };

    ret = poll(fds, 2, -1);
    assert(ret == 2);
    assert(fds[0].revents == POLLOUT);
    assert(fds[1].revents == POLLOUT);

    fds[0] = (struct pollfd){ .fd = 0, .events = POLLIN, .revents = 0 };
    time(&before);
    printf("time before = %lld\n", before);
    ret = poll(fds, 1, 2000);
    printf("ret = %d\n", ret);
    time(&now);
    printf("time now = %lld\n", now);
    assert(ret == 0);
    assert(now - before >= 2);

    sleep(1);
    time(&now);
    printf("time now = %lld\n", now);
    assert(now - before >= 3);

    return 0;
}
