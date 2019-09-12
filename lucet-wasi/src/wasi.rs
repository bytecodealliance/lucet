#![allow(clippy::too_many_arguments)]

pub use lucet_runtime::{self, vmctx::lucet_vmctx};
pub use wasi_common::*;

use lucet_runtime::lucet_hostcall_terminate;
use std::mem;
use std::rc::Rc;
use wasi_common::hostcalls::*;

lucet_runtime::lucet_hostcalls! {

#[no_mangle]
pub unsafe extern "C" fn __wasi_proc_exit(
    &mut _lucet_vmctx,
    rval: wasm32::__wasi_exitcode_t,
) -> ! {
    export_wasi_funcs();
    lucet_hostcall_terminate!(rval);
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_args_get(
    &mut lucet_ctx,
    argv_ptr: wasm32::uintptr_t,
    argv_buf: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    args_get(wasi_ctx, heap, argv_ptr, argv_buf)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_args_sizes_get(
    &mut lucet_ctx,
    argc_ptr: wasm32::uintptr_t,
    argv_buf_size_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    args_sizes_get(wasi_ctx, heap, argc_ptr, argv_buf_size_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_sched_yield(&mut _lucet_ctx,) -> wasm32::__wasi_errno_t {
    sched_yield()
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_clock_res_get(
    &mut lucet_ctx,
    clock_id: wasm32::__wasi_clockid_t,
    resolution_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let heap = &mut lucet_ctx.heap_mut();
    clock_res_get(heap, clock_id, resolution_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_clock_time_get(
    &mut lucet_ctx,
    clock_id: wasm32::__wasi_clockid_t,
    precision: wasm32::__wasi_timestamp_t,
    time_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let heap = &mut lucet_ctx.heap_mut();
    clock_time_get(heap, clock_id, precision, time_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_environ_get(
    &mut lucet_ctx,
    environ_ptr: wasm32::uintptr_t,
    environ_buf: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    environ_get(wasi_ctx, heap, environ_ptr, environ_buf)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_environ_sizes_get(
    &mut lucet_ctx,
    environ_count_ptr: wasm32::uintptr_t,
    environ_size_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    environ_sizes_get(wasi_ctx, heap, environ_count_ptr, environ_size_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_close(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &mut lucet_ctx.get_embed_ctx_mut::<WasiCtx>();
    fd_close(wasi_ctx, fd)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_fdstat_get(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
    fdstat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_fdstat_get(wasi_ctx, heap, fd, fdstat_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_fdstat_set_flags(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
    fdflags: wasm32::__wasi_fdflags_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    fd_fdstat_set_flags(wasi_ctx, fd, fdflags)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_tell(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_tell(wasi_ctx, heap, fd, offset)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_seek(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::__wasi_filedelta_t,
    whence: wasm32::__wasi_whence_t,
    newoffset: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_seek(wasi_ctx, heap, fd, offset, whence, newoffset)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_prestat_get(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
    prestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_prestat_get(wasi_ctx, heap, fd, prestat_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_prestat_dir_name(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_prestat_dir_name(wasi_ctx, heap, fd, path_ptr, path_len)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_read(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    nread: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &mut lucet_ctx.get_embed_ctx_mut::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_read(wasi_ctx, heap, fd, iovs_ptr, iovs_len, nread)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_write(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    nwritten: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &mut lucet_ctx.get_embed_ctx_mut::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_write(wasi_ctx, heap, fd, iovs_ptr, iovs_len, nwritten)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_path_open(
    &mut lucet_ctx,
    dirfd: wasm32::__wasi_fd_t,
    dirflags: wasm32::__wasi_lookupflags_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
    oflags: wasm32::__wasi_oflags_t,
    fs_rights_base: wasm32::__wasi_rights_t,
    fs_rights_inheriting: wasm32::__wasi_rights_t,
    fs_flags: wasm32::__wasi_fdflags_t,
    fd_out_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
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
    buf_ptr: wasm32::uintptr_t,
    buf_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    let heap = &mut lucet_ctx.heap_mut();
    random_get(heap, buf_ptr, buf_len)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_poll_oneoff(
    &mut lucet_ctx,
    input: wasm32::uintptr_t,
    output: wasm32::uintptr_t,
    nsubscriptions: wasm32::size_t,
    nevents: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let heap = &mut lucet_ctx.heap_mut();
    poll_oneoff(heap, input, output, nsubscriptions, nevents)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_filestat_get(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
    filestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_filestat_get(wasi_ctx, heap, fd, filestat_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_path_filestat_get(
    &mut lucet_ctx,
    dirfd: wasm32::__wasi_fd_t,
    dirflags: wasm32::__wasi_lookupflags_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
    filestat_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
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
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    path_create_directory(wasi_ctx, heap, dirfd, path_ptr, path_len)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_path_unlink_file(
    &mut lucet_ctx,
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    path_unlink_file(wasi_ctx, heap, dirfd, path_ptr, path_len)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_allocate(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::__wasi_filesize_t,
    len: wasm32::__wasi_filesize_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    fd_allocate(wasi_ctx, fd, offset, len)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_advise(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
    offset: wasm32::__wasi_filesize_t,
    len: wasm32::__wasi_filesize_t,
    advice: wasm32::__wasi_advice_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    fd_advise(wasi_ctx, fd, offset, len, advice)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_datasync(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    fd_datasync(wasi_ctx, fd)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_sync(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    fd_sync(wasi_ctx, fd)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_fdstat_set_rights(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
    fs_rights_base: wasm32::__wasi_rights_t,
    fs_rights_inheriting: wasm32::__wasi_rights_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &mut lucet_ctx.get_embed_ctx_mut::<WasiCtx>();
    fd_fdstat_set_rights(wasi_ctx, fd, fs_rights_base, fs_rights_inheriting)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_filestat_set_size(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
    st_size: wasm32::__wasi_filesize_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    fd_filestat_set_size(wasi_ctx, fd, st_size)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_filestat_set_times(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
    st_atim: wasm32::__wasi_timestamp_t,
    st_mtim: wasm32::__wasi_timestamp_t,
    fst_flags: wasm32::__wasi_fstflags_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    fd_filestat_set_times(wasi_ctx, fd, st_atim, st_mtim, fst_flags)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_pread(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    offset: wasm32::__wasi_filesize_t,
    nread: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_pread(wasi_ctx, heap, fd, iovs_ptr, iovs_len, offset, nread)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_pwrite(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
    iovs_ptr: wasm32::uintptr_t,
    iovs_len: wasm32::size_t,
    offset: wasm32::__wasi_filesize_t,
    nwritten: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_pwrite(wasi_ctx, heap, fd, iovs_ptr, iovs_len, offset, nwritten)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_readdir(
    &mut lucet_ctx,
    fd: wasm32::__wasi_fd_t,
    buf: wasm32::uintptr_t,
    buf_len: wasm32::size_t,
    cookie: wasm32::__wasi_dircookie_t,
    bufused: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    fd_readdir(wasi_ctx, heap, fd, buf, buf_len, cookie, bufused)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_fd_renumber(
    &mut lucet_ctx,
    from: wasm32::__wasi_fd_t,
    to: wasm32::__wasi_fd_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &mut lucet_ctx.get_embed_ctx_mut::<WasiCtx>();
    fd_renumber(wasi_ctx, from, to)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_path_filestat_set_times(
    &mut lucet_ctx,
    dirfd: wasm32::__wasi_fd_t,
    dirflags: wasm32::__wasi_lookupflags_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
    st_atim: wasm32::__wasi_timestamp_t,
    st_mtim: wasm32::__wasi_timestamp_t,
    fst_flags: wasm32::__wasi_fstflags_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    path_filestat_set_times(
        wasi_ctx, heap, dirfd, dirflags, path_ptr, path_len, st_atim, st_mtim, fst_flags,
    )
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_path_link(
    &mut lucet_ctx,
    old_fd: wasm32::__wasi_fd_t,
    old_flags: wasm32::__wasi_lookupflags_t,
    old_path_ptr: wasm32::uintptr_t,
    old_path_len: wasm32::size_t,
    new_fd: wasm32::__wasi_fd_t,
    new_path_ptr: wasm32::uintptr_t,
    new_path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
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
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
    buf_ptr: wasm32::uintptr_t,
    buf_len: wasm32::size_t,
    bufused: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    path_readlink(
        wasi_ctx, heap, dirfd, path_ptr, path_len, buf_ptr, buf_len, bufused,
    )
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_path_remove_directory(
    &mut lucet_ctx,
    dirfd: wasm32::__wasi_fd_t,
    path_ptr: wasm32::uintptr_t,
    path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    let wasi_ctx = &lucet_ctx.get_embed_ctx::<WasiCtx>();
    let heap = &mut lucet_ctx.heap_mut();
    path_remove_directory(wasi_ctx, heap, dirfd, path_ptr, path_len)
}

#[no_mangle]
pub unsafe extern "C" fn __wasi_path_rename(
    &mut lucet_ctx,
    old_dirfd: wasm32::__wasi_fd_t,
    old_path_ptr: wasm32::uintptr_t,
    old_path_len: wasm32::size_t,
    new_dirfd: wasm32::__wasi_fd_t,
    new_path_ptr: wasm32::uintptr_t,
    new_path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
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
    old_path_ptr: wasm32::uintptr_t,
    old_path_len: wasm32::size_t,
    dir_fd: wasm32::__wasi_fd_t,
    new_path_ptr: wasm32::uintptr_t,
    new_path_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
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
