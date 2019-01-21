#define __SYSCALL_LL_E(x) \
((union { long long ll; long l[2]; }){ .ll = x }).l[0], \
((union { long long ll; long l[2]; }){ .ll = x }).l[1]
#define __SYSCALL_LL_O(x) 0, __SYSCALL_LL_E((x))

long __syscall0(long n);
long __syscall1(long n, long a);
long __syscall2(long n, long a, long b);
long __syscall3(long n, long a, long b, long c);
long __syscall4(long n, long a, long b, long c, long d);
long __syscall5(long n, long a, long b, long c, long d, long e);
long __syscall6(long n, long a, long b, long c, long d, long e, long f);

// HACK other architectures don't do this.
#include "../../src/internal/syscall.h"
