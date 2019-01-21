import os
import glob

def musl_srcs(musl_root):

    def allow_from(d, fs):
        return [ os.path.join(musl_root, 'src', d, f) for f in fs ]

    def disallow_from(d, fs):
        sources = []
        base = os.path.join(musl_root, 'src', d)
        for f in glob.glob(os.path.join(base, '*.c')):
            if os.path.basename(f) in fs:
                continue
            sources.append(os.path.join(base, f))
        return sources

    l = [
    # aio depends on pthreads, so we won't support it
    allow_from('aio', []),

    # not bothering with complex math for now
    allow_from('complex', []),

    # conf functions seem totally useless
    allow_from('conf', []),

    # you can find much more sensible crypto implementations if you really care
    # about it
    allow_from('crypt', []),

    # ctype has some pretty essential contents
    # Blacklist all wide char related functions, which
    # depend on pthread_self for locale information
    disallow_from('ctype', [
        '__ctype_get_mb_cur_max.c',
        'iswalnum.c',
        'iswalpha.c',
        'iswblank.c',
        'iswcntrl.c',
        'iswctype.c',
        'iswdigit.c',
        'iswgraph.c',
        'iswlower.c',
        'iswprint.c',
        'iswpunct.c',
        'iswspace.c',
        'iswupper.c',
        'iswxdigit.c',
        'wcswidth.c',
        'wctrans.c',
        'towctrans.c',
        'wcwidth.c',
    ]),
    # Don't support dirent: we dont have a filesystem
    allow_from('dirent', []),

    # Don't support an env
    allow_from('env', []),

    # Errno storage and conversion to strings. We could probably
    # replace strerror with a function that returns const "unimplemented"
    # but optimize that later I guess.
    allow_from('errno', [
        '__errno_location.c',
        'strerror.c',
    ]),

    # We only want assert from here - exit and abort have a custom impl
    # in the wasm32_rt
    allow_from('exit', ['assert.c']),

    # Don't support fcntl: we dont have a filesystem
    allow_from('fcntl', []),

    # Floating point environment functions are required for math
    disallow_from('fenv', []),

    allow_from('internal', [
        'floatscan.c',
        'intscan.c',
        'libc.c',
        'procfdname.c',
        'shgetc.c',
        'vdso.c',
        'version.c',
    ]),

    # Don't support ipc
    allow_from('ipc', []),

    # Don't support dynamic loading
    allow_from('ldso', []),

    # Don't support legacy
    allow_from('legacy', []),

    # Don't support linux
    allow_from('linux', []),

    # Don't support locale
    allow_from('locale', []),

    # Memory allocation is mostly supported by a malloc/free impl in
    # wasm32_rt. These additional functions just call those functions:
    allow_from('malloc', [
        'malloc_usable_size.c',
        'memalign.c',
        'posix_memalign.c',
    ]),

    # Support all of math
    disallow_from('math', []),

    # Its possible to support some of multibyte, but most of it
    # depends on pthread_self for locale, so we'll leave it out until we
    # have a way to support it properly
    allow_from('multibyte', [
        'wctomb.c', # required by vfprintf
        'wcrtomb.c', # required by vfprintf, FIXME THIS REQUIRES LOCALE!!!!!
    ]),

    # Don't support network
    allow_from('network', []),

    # Don't support passwd
    allow_from('passwd', []),

    # Support all of prng
    disallow_from('prng', []),

    # Don't support process
    allow_from('process', []),

    # Don't support regex
    allow_from('regex', []),

    # Don't support sched
    allow_from('sched', []),

    # Don't support search
    allow_from('search', []),

    # Don't support select
    allow_from('select', []),

    # Don't support setjmp
    allow_from('setjmp', []),

    # Don't support signal
    allow_from('signal', []),

    # Don't support stat
    allow_from('stat', []),

    # Its tough to support stdio without threads, unix syscalls, and memory
    # allocation. we customize the output functions elsewhere.
    disallow_from('stdio', [
        '__fdopen.c', # malloc
        '__fopen_rb_ca.c', # syscall
        '__lockfile.c', # pthreads
        '__stdio_close.c', # malloc
        '__stdio_read.c', # needs stub implementation
        '__stdio_seek.c', # needs stub implementation
        '__stdio_write.c', # needs custom implementation
        '__stdout_write.c', # needs custom implementation
        'asprintf.c', # malloc
        'fclose.c', # malloc
        'fgetln.c', # malloc via getdelim
        'fgetwc.c', # widechar
        'fgetws.c', # widechar
        'flockfile.c', # pthreads
        'fmemopen.c', # malloc
        'fopen.c', # malloc
        'fputwc.c', # widechar
        'fputws.c', # widechar
        'freopen.c', # syscall
        'fscanf.c', # malloc via vfscanf
        'ftrylockfile.c', # pthreads
        'funlockfile.c', # pthreads
        'fwide.c', # widechar
        'fwprintf.c', # widechar
        'fwscanf.c', # widechar
        'getdelim.c', # malloc
        'getline.c', # malloc via getdelim
        'getw.c', # widechar
        'getwc.c', # widechar
        'getwchar.c', # widechar
        'open_memstream.c', # malloc
        'open_wmemstream.c', # malloc, widechar
        'pclose.c', # syscall
        'perror.c', # syscall
        'popen.c', # syscall
        'putw.c', # widechar
        'putwc.c', # widechar
        'putwchar.c', # widechar
        'remove.c', # syscall
        'rename.c', # syscall
        'scanf.c', # malloc via vfscanf
        'sscanf.c', # malloc via vfscanf
        'swprintf.c', # widechar
        'swscanf.c', # widechar
        'tempnam.c', # syscall
        'tmpfile.c', # syscall
        'tmpnam.c', # syscall
        'ungetwc.c', # widechar
        'vasprintf.c', # malloc
        'vfscanf.c', # malloc
        'vfwprintf.c', # widechar
        'vfwscanf.c', # widechar
        'vscanf.c', # malloc via vfscanf
        'vsscanf.c', # malloc via vfscanf
        'vswprintf.c', # widechar
        'vswscanf.c', # widechar
        'vwprintf.c', # widechar
        'vwscanf.c', # widechar
        'wprintf.c', # widechar
        'wscanf.c', # widechar
    ]),


    disallow_from('stdlib', [
        'wcstod.c', # widechar
        'wcstol.c', # widechar
    ]),

    disallow_from('string', [
        'strdup.c', # malloc
        'strndup.c', # malloc
        'strsignal.c', # locale
        'wcpcpy.c', # widechar (for rest)
        'wcpncpy.c',
        'wcscasecmp.c',
        'wcscasecmp_l.c',
        'wcscat.c',
        'wcschr.c',
        'wcscmp.c',
        'wcscpy.c',
        'wcscspn.c',
        'wcsdup.c',
        'wcslen.c',
        'wcsncasecmp.c',
        'wcsncasecmp_l.c',
        'wcsncat.c',
        'wcsncmp.c',
        'wcsncpy.c',
        'wcsnlen.c',
        'wcspbrk.c',
        'wcsrchr.c',
        'wcsspn.c',
        'wcsstr.c',
        'wcstok.c',
        'wcswcs.c',
        'wmemchr.c',
        'wmemcmp.c',
        'wmemcpy.c',
        'wmemmove.c',
        'wmemset.c',
    ]),

    # Don't support temp
    allow_from('temp', []),

    # Don't support termios
    allow_from('termios', []),

    # Don't support threading
    allow_from('thread', []),

    # Don't support time
    allow_from('time', []),

    allow_from('unistd', [
        '_exit.c',
    ]),
    ]
    # The above is a list of lists, flatten and return it
    return [item for sublist in l for item in sublist]


"""
    disallow_from(os.path.join('..', '..', 'compiler-rt', 'lib', 'builtins'), [
    ]),

    # wasm32_rt provides Fastly liblucet-runtime-c specific implementations of various
    # functions required by musl where either the builtin or the musl version
    # is inappropriate
    disallow_from(os.path.join('..', 'wasm32_rt'), []),
"""

def wasm_srcs(wasm_root):

    def disallow_from(d, fs):
        headers = []
        base = os.path.join(wasm_root, d)
        for f in glob.glob(os.path.join(base, '*.c')):
            if os.path.basename(f) in fs:
                continue
            headers.append(os.path.join(base, f))
        return headers

    return disallow_from('wasm32_rt', [])

def compiler_rt_srcs(compiler_rt_root):
    def disallow_from(d, fs):
        headers = []
        base = os.path.join(compiler_rt_root, d)
        for f in glob.glob(os.path.join(base, '*.c')):
            if os.path.basename(f) in fs:
                continue
            headers.append(os.path.join(base, f))
        return headers

    # Compiler-rt builtins are typically provided as `libgcc.a` but for wasm
    # we need to link with them as llvm bitcode, so we build them as part of
    # this as well.
    # Blacklisted files all deal with concepts not relevant to wasm - typically
    # threads.

    return disallow_from(os.path.join('lib', 'builtins'), [
        'atomic.c',
        'atomic_flag_clear.c',
        'atomic_flag_clear_explicit.c',
        'atomic_flag_test_and_set.c',
        'atomic_flag_test_and_set_explicit.c',
        'atomic_signal_fence.c',
        'atomic_thread_fence.c',
        'clear_cache.c',
        'cpu_model.c',
        'emutls.c',
        'enable_execute_stack.c',
        'eprintf.c',
        'gcc_personality_v0.c',
        'int_util.c',
        'mingw_fixfloat.c',
        'os_version_check.c',
        'trampoline_setup.c',
    ])


def host_srcs(host_root):
    return [
        os.path.join(host_root, 'src', 'lucet_libc.c')
    ]
