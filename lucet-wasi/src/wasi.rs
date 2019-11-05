#![allow(clippy::too_many_arguments)]

pub use lucet_runtime::{self, vmctx::lucet_vmctx};
pub use wasi_common::{wasi, wasi32, WasiCtx};

use lucet_runtime::lucet_hostcall_terminate;
use std::mem;
use std::rc::Rc;
use wasi_common::hostcalls::*;

lucet_runtime::lucet_hostcalls! {

#[no_mangle]
pub unsafe extern "C" fn __wasi_proc_exit(
    &mut _lucet_vmctx,
    rval: wasi::__wasi_exitcode_t,
) -> ! {
    export_wasi_funcs();
    lucet_hostcall_terminate!(rval);
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_args_get(
    &mut lucet_ctx,
    argv_ptr: wasi32::uintptr_t,
    argv_buf: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    args_get(wasi_ctx, heap, argv_ptr, argv_buf)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_args_sizes_get(
    &mut lucet_ctx,
    argc_ptr: wasi32::uintptr_t,
    argv_buf_size_ptr: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    args_sizes_get(wasi_ctx, heap, argc_ptr, argv_buf_size_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_sched_yield(&mut _lucet_ctx,) -> wasi::__wasi_errno_t {
    sched_yield()
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_clock_res_get(
    &mut lucet_ctx,
    clock_id: wasi::__wasi_clockid_t,
    resolution_ptr: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let heap = &mut lucet_ctx.heap_mut();
    clock_res_get(heap, clock_id, resolution_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_clock_time_get(
    &mut lucet_ctx,
    clock_id: wasi::__wasi_clockid_t,
    precision: wasi::__wasi_timestamp_t,
    time_ptr: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let heap = &mut lucet_ctx.heap_mut();
    clock_time_get(heap, clock_id, precision, time_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_environ_get(
    &mut lucet_ctx,
    environ_ptr: wasi32::uintptr_t,
    environ_buf: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    environ_get(wasi_ctx, heap, environ_ptr, environ_buf)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_environ_sizes_get(
    &mut lucet_ctx,
    environ_count_ptr: wasi32::uintptr_t,
    environ_size_ptr: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    environ_sizes_get(wasi_ctx, heap, environ_count_ptr, environ_size_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_close(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &mut lucet_ctx.get_embed_ctx_mut::<WasiCtx>();
    fd_close(wasi_ctx, fd)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_fdstat_get(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
    fdstat_ptr: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_fdstat_get(wasi_ctx, heap, fd, fdstat_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_fdstat_set_flags(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
    fdflags: wasi::__wasi_fdflags_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    fd_fdstat_set_flags(wasi_ctx, fd, fdflags)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_tell(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
    offset: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &mut lucet_ctx.get_embed_ctx_mut::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_tell(wasi_ctx, heap, fd, offset)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_seek(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
    offset: wasi::__wasi_filedelta_t,
    whence: wasi::__wasi_whence_t,
    newoffset: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &mut lucet_ctx.get_embed_ctx_mut::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_seek(wasi_ctx, heap, fd, offset, whence, newoffset)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_prestat_get(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
    prestat_ptr: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_prestat_get(wasi_ctx, heap, fd, prestat_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_prestat_dir_name(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
    path_ptr: wasi32::uintptr_t,
    path_len: wasi32::size_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_prestat_dir_name(wasi_ctx, heap, fd, path_ptr, path_len)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_read(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
    iovs_ptr: wasi32::uintptr_t,
    iovs_len: wasi32::size_t,
    nread: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &mut lucet_ctx.get_embed_ctx_mut::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_read(wasi_ctx, heap, fd, iovs_ptr, iovs_len, nread)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_write(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
    iovs_ptr: wasi32::uintptr_t,
    iovs_len: wasi32::size_t,
    nwritten: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &mut lucet_ctx.get_embed_ctx_mut::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_write(wasi_ctx, heap, fd, iovs_ptr, iovs_len, nwritten)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_path_open(
    &mut lucet_ctx,
    dirfd: wasi::__wasi_fd_t,
    dirflags: wasi::__wasi_lookupflags_t,
    path_ptr: wasi32::uintptr_t,
    path_len: wasi32::size_t,
    oflags: wasi::__wasi_oflags_t,
    fs_rights_base: wasi::__wasi_rights_t,
    fs_rights_inheriting: wasi::__wasi_rights_t,
    fs_flags: wasi::__wasi_fdflags_t,
    fd_out_ptr: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &mut lucet_ctx.get_embed_ctx_mut::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    path_open(
        wasi_ctx,
        heap,
        dirfd,
        dirflags,
        path_ptr,
        path_len,
        oflags,
        fs_rights_base,
        fs_rights_inheriting,
        fs_flags,
        fd_out_ptr,
    )
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_random_get(
    &mut lucet_ctx,
    buf_ptr: wasi32::uintptr_t,
    buf_len: wasi32::size_t,
) -> wasi::__wasi_errno_t {
    let heap = &mut lucet_ctx.heap_mut();
    random_get(heap, buf_ptr, buf_len)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_poll_oneoff(
    &mut lucet_ctx,
    input: wasi32::uintptr_t,
    output: wasi32::uintptr_t,
    nsubscriptions: wasi32::size_t,
    nevents: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    poll_oneoff(wasi_ctx, heap, input, output, nsubscriptions, nevents)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_filestat_get(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
    filestat_ptr: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_filestat_get(wasi_ctx, heap, fd, filestat_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_path_filestat_get(
    &mut lucet_ctx,
    dirfd: wasi::__wasi_fd_t,
    dirflags: wasi::__wasi_lookupflags_t,
    path_ptr: wasi32::uintptr_t,
    path_len: wasi32::size_t,
    filestat_ptr: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    path_filestat_get(
        wasi_ctx,
        heap,
        dirfd,
        dirflags,
        path_ptr,
        path_len,
        filestat_ptr,
    )
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_path_create_directory(
    &mut lucet_ctx,
    dirfd: wasi::__wasi_fd_t,
    path_ptr: wasi32::uintptr_t,
    path_len: wasi32::size_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    path_create_directory(wasi_ctx, heap, dirfd, path_ptr, path_len)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_path_unlink_file(
    &mut lucet_ctx,
    dirfd: wasi::__wasi_fd_t,
    path_ptr: wasi32::uintptr_t,
    path_len: wasi32::size_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    path_unlink_file(wasi_ctx, heap, dirfd, path_ptr, path_len)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_allocate(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
    offset: wasi::__wasi_filesize_t,
    len: wasi::__wasi_filesize_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    fd_allocate(wasi_ctx, fd, offset, len)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_advise(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
    offset: wasi::__wasi_filesize_t,
    len: wasi::__wasi_filesize_t,
    advice: wasi::__wasi_advice_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    fd_advise(wasi_ctx, fd, offset, len, advice)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_datasync(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    fd_datasync(wasi_ctx, fd)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_sync(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    fd_sync(wasi_ctx, fd)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_fdstat_set_rights(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
    fs_rights_base: wasi::__wasi_rights_t,
    fs_rights_inheriting: wasi::__wasi_rights_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &mut lucet_ctx.get_embed_ctx_mut::<WasiCtx>();
    fd_fdstat_set_rights(wasi_ctx, fd, fs_rights_base, fs_rights_inheriting)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_filestat_set_size(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
    st_size: wasi::__wasi_filesize_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    fd_filestat_set_size(wasi_ctx, fd, st_size)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_filestat_set_times(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
    st_atim: wasi::__wasi_timestamp_t,
    st_mtim: wasi::__wasi_timestamp_t,
    fst_flags: wasi::__wasi_fstflags_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    fd_filestat_set_times(wasi_ctx, fd, st_atim, st_mtim, fst_flags)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_pread(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
    iovs_ptr: wasi32::uintptr_t,
    iovs_len: wasi32::size_t,
    offset: wasi::__wasi_filesize_t,
    nread: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_pread(wasi_ctx, heap, fd, iovs_ptr, iovs_len, offset, nread)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_pwrite(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
    iovs_ptr: wasi32::uintptr_t,
    iovs_len: wasi32::size_t,
    offset: wasi::__wasi_filesize_t,
    nwritten: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_pwrite(wasi_ctx, heap, fd, iovs_ptr, iovs_len, offset, nwritten)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_readdir(
    &mut lucet_ctx,
    fd: wasi::__wasi_fd_t,
    buf: wasi32::uintptr_t,
    buf_len: wasi32::size_t,
    cookie: wasi::__wasi_dircookie_t,
    bufused: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &mut lucet_ctx.get_embed_ctx_mut::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_readdir(wasi_ctx, heap, fd, buf, buf_len, cookie, bufused)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_renumber(
    &mut lucet_ctx,
    from: wasi::__wasi_fd_t,
    to: wasi::__wasi_fd_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &mut lucet_ctx.get_embed_ctx_mut::<WasiCtx>();
    fd_renumber(wasi_ctx, from, to)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_path_filestat_set_times(
    &mut lucet_ctx,
    dirfd: wasi::__wasi_fd_t,
    dirflags: wasi::__wasi_lookupflags_t,
    path_ptr: wasi32::uintptr_t,
    path_len: wasi32::size_t,
    st_atim: wasi::__wasi_timestamp_t,
    st_mtim: wasi::__wasi_timestamp_t,
    fst_flags: wasi::__wasi_fstflags_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    path_filestat_set_times(
        wasi_ctx, heap, dirfd, dirflags, path_ptr, path_len, st_atim, st_mtim, fst_flags,
    )
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_path_link(
    &mut lucet_ctx,
    old_fd: wasi::__wasi_fd_t,
    old_flags: wasi::__wasi_lookupflags_t,
    old_path_ptr: wasi32::uintptr_t,
    old_path_len: wasi32::size_t,
    new_fd: wasi::__wasi_fd_t,
    new_path_ptr: wasi32::uintptr_t,
    new_path_len: wasi32::size_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    path_link(
        wasi_ctx,
        heap,
        old_fd,
        old_flags,
        old_path_ptr,
        old_path_len,
        new_fd,
        new_path_ptr,
        new_path_len,
    )
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_path_readlink(
    &mut lucet_ctx,
    dirfd: wasi::__wasi_fd_t,
    path_ptr: wasi32::uintptr_t,
    path_len: wasi32::size_t,
    buf_ptr: wasi32::uintptr_t,
    buf_len: wasi32::size_t,
    bufused: wasi32::uintptr_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    path_readlink(
        wasi_ctx, heap, dirfd, path_ptr, path_len, buf_ptr, buf_len, bufused,
    )
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_path_remove_directory(
    &mut lucet_ctx,
    dirfd: wasi::__wasi_fd_t,
    path_ptr: wasi32::uintptr_t,
    path_len: wasi32::size_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    path_remove_directory(wasi_ctx, heap, dirfd, path_ptr, path_len)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_path_rename(
    &mut lucet_ctx,
    old_dirfd: wasi::__wasi_fd_t,
    old_path_ptr: wasi32::uintptr_t,
    old_path_len: wasi32::size_t,
    new_dirfd: wasi::__wasi_fd_t,
    new_path_ptr: wasi32::uintptr_t,
    new_path_len: wasi32::size_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    path_rename(
        wasi_ctx,
        heap,
        old_dirfd,
        old_path_ptr,
        old_path_len,
        new_dirfd,
        new_path_ptr,
        new_path_len,
    )
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_path_symlink(
    &mut lucet_ctx,
    old_path_ptr: wasi32::uintptr_t,
    old_path_len: wasi32::size_t,
    dir_fd: wasi::__wasi_fd_t,
    new_path_ptr: wasi32::uintptr_t,
    new_path_len: wasi32::size_t,
) -> wasi::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    path_symlink(
        wasi_ctx,
        heap,
        old_path_ptr,
        old_path_len,
        dir_fd,
        new_path_ptr,
        new_path_len,
    )
}

}

pub fn export_wasi_funcs() {
    let funcs: &[*const extern "C" fn()] = &[
        __wasi_args_get as _,
        __wasi_args_sizes_get as _,
        __wasi_sched_yield as _,
        __wasi_clock_res_get as _,
        __wasi_clock_time_get as _,
        __wasi_environ_get as _,
        __wasi_environ_sizes_get as _,
        __wasi_fd_close as _,
        __wasi_fd_fdstat_get as _,
        __wasi_fd_fdstat_set_flags as _,
        __wasi_fd_tell as _,
        __wasi_fd_seek as _,
        __wasi_fd_prestat_get as _,
        __wasi_fd_prestat_dir_name as _,
        __wasi_fd_read as _,
        __wasi_fd_write as _,
        __wasi_path_open as _,
        __wasi_random_get as _,
        __wasi_poll_oneoff as _,
        __wasi_fd_filestat_get as _,
        __wasi_path_filestat_get as _,
        __wasi_path_create_directory as _,
        __wasi_path_unlink_file as _,
        __wasi_fd_allocate as _,
        __wasi_fd_advise as _,
        __wasi_fd_datasync as _,
        __wasi_fd_sync as _,
        __wasi_fd_fdstat_set_rights as _,
        __wasi_fd_filestat_set_size as _,
        __wasi_fd_filestat_set_times as _,
        __wasi_fd_pread as _,
        __wasi_fd_pwrite as _,
        __wasi_fd_readdir as _,
        __wasi_fd_renumber as _,
        __wasi_path_filestat_set_times as _,
        __wasi_path_link as _,
        __wasi_path_readlink as _,
        __wasi_path_remove_directory as _,
        __wasi_path_rename as _,
        __wasi_path_symlink as _,
        __wasi_proc_exit as _,
    ];
    mem::forget(Rc::new(funcs));
}
