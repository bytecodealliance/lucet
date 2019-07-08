#![allow(non_camel_case_types)]
#![allow(unused_unsafe)]

use crate::ctx::WasiCtx;
use crate::memory::*;
use crate::{host, wasm32};

use cast::From as _0;
use lucet_runtime::lucet_hostcall_terminate;
use lucet_runtime::vmctx::Vmctx;

use nix::convert_ioctl_res;
use nix::libc::{self, c_int};
use std::cmp;
use std::mem::MaybeUninit;
use std::time::SystemTime;

// define the `fionread()` function, equivalent to `ioctl(fd, FIONREAD, *bytes)`
nix::ioctl_read_bad!(fionread, nix::libc::FIONREAD, c_int);

fn wasi_clock_to_relative_ns_delay(
    wasi_clock: host::__wasi_subscription_t___wasi_subscription_u___wasi_subscription_u_clock_t,
) -> u128 {
    if wasi_clock.flags != wasm32::__WASI_SUBSCRIPTION_CLOCK_ABSTIME {
        return wasi_clock.timeout as u128;
    }
    let now: u128 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Current date is before the epoch")
        .as_nanos();
    let deadline = wasi_clock.timeout as u128;
    deadline.saturating_sub(now)
}

#[derive(Debug, Copy, Clone)]
struct ClockEventData {
    delay: u128,
    userdata: host::__wasi_userdata_t,
}
#[derive(Debug, Copy, Clone)]
struct FdEventData {
    fd: c_int,
    type_: host::__wasi_eventtype_t,
    userdata: host::__wasi_userdata_t,
}

pub fn wasi_proc_exit(_vmctx: &mut Vmctx, rval: wasm32::__wasi_exitcode_t) -> ! {
    lucet_hostcall_terminate!(dec_exitcode(rval));
}

pub fn wasi_args_get(
    vmctx: &mut Vmctx,
    argv_ptr: wasm32::uintptr_t,
    argv_buf: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let ctx = vmctx.get_embed_ctx::<WasiCtx>();

    let mut argv_buf_offset = 0;
    let mut argv = vec![];

    for arg in ctx.args.iter() {
        let arg_bytes = arg.as_bytes_with_nul();
        let arg_ptr = argv_buf + argv_buf_offset;

        if let Err(e) = enc_slice_of(vmctx, arg_bytes, arg_ptr) {
            return enc_errno(e);
        }

        argv.push(arg_ptr);

        argv_buf_offset = if let Some(new_offset) = argv_buf_offset.checked_add(
            wasm32::uintptr_t::cast(arg_bytes.len())
                .expect("cast overflow would have been caught by `enc_slice_of` above"),
        ) {
            new_offset
        } else {
            return wasm32::__WASI_EOVERFLOW;
        }
    }
    enc_slice_of(vmctx, argv.as_slice(), argv_ptr)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(|e| e)
}

pub fn wasi_args_sizes_get(
    vmctx: &mut Vmctx,
    argc_ptr: wasm32::uintptr_t,
    argv_buf_size_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let ctx = vmctx.get_embed_ctx::<WasiCtx>();

    let argc = ctx.args.len();
    let argv_size = ctx
        .args
        .iter()
        .map(|arg| arg.as_bytes_with_nul().len())
        .sum();
    if let Err(e) = enc_usize_byref(vmctx, argc_ptr, argc) {
        return enc_errno(e);
    }
    if let Err(e) = enc_usize_byref(vmctx, argv_buf_size_ptr, argv_size) {
        return enc_errno(e);
    }
    wasm32::__WASI_ESUCCESS
}

pub fn wasi_sched_yield(_vmctx: &mut Vmctx) -> wasm32::__wasi_errno_t {
    unsafe { libc::sched_yield() };
    wasm32::__WASI_ESUCCESS
}

pub fn wasi_clock_res_get(
    vmctx: &mut Vmctx,
    clock_id: wasm32::__wasi_clockid_t,
    resolution_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    // convert the supported clocks to the libc types, or return EINVAL
    let clock_id = match dec_clockid(clock_id) {
        host::__WASI_CLOCK_REALTIME => libc::CLOCK_REALTIME,
        host::__WASI_CLOCK_MONOTONIC => libc::CLOCK_MONOTONIC,
        host::__WASI_CLOCK_PROCESS_CPUTIME_ID => libc::CLOCK_PROCESS_CPUTIME_ID,
        host::__WASI_CLOCK_THREAD_CPUTIME_ID => libc::CLOCK_THREAD_CPUTIME_ID,
        _ => return wasm32::__WASI_EINVAL,
    };

    // no `nix` wrapper for clock_getres, so we do it ourselves
    let mut timespec = MaybeUninit::<libc::timespec>::uninit();
    let res = unsafe { libc::clock_getres(clock_id, timespec.as_mut_ptr()) };
    if res != 0 {
        return wasm32::errno_from_nix(nix::errno::Errno::last());
    }
    let timespec = unsafe { timespec.assume_init() };

    // convert to nanoseconds, returning EOVERFLOW in case of overflow; this is freelancing a bit
    // from the spec but seems like it'll be an unusual situation to hit
    (timespec.tv_sec as host::__wasi_timestamp_t)
        .checked_mul(1_000_000_000)
        .and_then(|sec_ns| sec_ns.checked_add(timespec.tv_nsec as host::__wasi_timestamp_t))
        .map(|resolution| {
            // a supported clock can never return zero; this case will probably never get hit, but
            // make sure we follow the spec
            if resolution == 0 {
                wasm32::__WASI_EINVAL
            } else {
                enc_timestamp_byref(vmctx, resolution_ptr, resolution)
                    .map(|_| wasm32::__WASI_ESUCCESS)
                    .unwrap_or_else(|e| e)
            }
        })
        .unwrap_or(wasm32::__WASI_EOVERFLOW)
}

pub fn wasi_clock_time_get(
    vmctx: &mut Vmctx,
    clock_id: wasm32::__wasi_clockid_t,
    // ignored for now, but will be useful once we put optional limits on precision to reduce side
    // channels
    _precision: wasm32::__wasi_timestamp_t,
    time_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    // convert the supported clocks to the libc types, or return EINVAL
    let clock_id = match dec_clockid(clock_id) {
        host::__WASI_CLOCK_REALTIME => libc::CLOCK_REALTIME,
        host::__WASI_CLOCK_MONOTONIC => libc::CLOCK_MONOTONIC,
        host::__WASI_CLOCK_PROCESS_CPUTIME_ID => libc::CLOCK_PROCESS_CPUTIME_ID,
        host::__WASI_CLOCK_THREAD_CPUTIME_ID => libc::CLOCK_THREAD_CPUTIME_ID,
        _ => return wasm32::__WASI_EINVAL,
    };

    // no `nix` wrapper for clock_getres, so we do it ourselves
    let mut timespec = MaybeUninit::<libc::timespec>::uninit();
    let res = unsafe { libc::clock_gettime(clock_id, timespec.as_mut_ptr()) };
    if res != 0 {
        return wasm32::errno_from_nix(nix::errno::Errno::last());
    }
    let timespec = unsafe { timespec.assume_init() };
    // convert to nanoseconds, returning EOVERFLOW in case of overflow; this is freelancing a bit
    // from the spec but seems like it'll be an unusual situation to hit
    (timespec.tv_sec as host::__wasi_timestamp_t)
        .checked_mul(1_000_000_000)
        .and_then(|sec_ns| sec_ns.checked_add(timespec.tv_nsec as host::__wasi_timestamp_t))
        .map(|time| {
            enc_timestamp_byref(vmctx, time_ptr, time)
                .map(|_| wasm32::__WASI_ESUCCESS)
                .unwrap_or_else(|e| e)
        })
        .unwrap_or(wasm32::__WASI_EOVERFLOW)
}

pub fn wasi_environ_get(
    vmctx: &mut Vmctx,
    environ_ptr: wasm32::uintptr_t,
    environ_buf: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let ctx = vmctx.get_embed_ctx::<WasiCtx>();

    let mut environ_buf_offset = 0;
    let mut environ = vec![];

    for pair in ctx.env.iter() {
        let env_bytes = pair.as_bytes_with_nul();
        let env_ptr = environ_buf + environ_buf_offset;

        if let Err(e) = enc_slice_of(vmctx, env_bytes, env_ptr) {
            return enc_errno(e);
        }

        environ.push(env_ptr);

        environ_buf_offset = if let Some(new_offset) = environ_buf_offset.checked_add(
            wasm32::uintptr_t::cast(env_bytes.len())
                .expect("cast overflow would have been caught by `enc_slice_of` above"),
        ) {
            new_offset
        } else {
            return wasm32::__WASI_EOVERFLOW;
        }
    }
    enc_slice_of(vmctx, environ.as_slice(), environ_ptr)
        .map(|_| wasm32::__WASI_ESUCCESS)
        .unwrap_or_else(|e| e)
}

pub fn wasi_environ_sizes_get(
    vmctx: &mut Vmctx,
    environ_count_ptr: wasm32::uintptr_t,
    environ_size_ptr: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    let ctx = vmctx.get_embed_ctx::<WasiCtx>();

    let environ_count = ctx.env.len();
    if let Some(environ_size) = ctx.env.iter().try_fold(0, |acc: u32, pair| {
        acc.checked_add(pair.as_bytes_with_nul().len() as u32)
    }) {
        if let Err(e) = enc_usize_byref(vmctx, environ_count_ptr, environ_count) {
            return enc_errno(e);
        }
        if let Err(e) = enc_usize_byref(vmctx, environ_size_ptr, environ_size as usize) {
            return enc_errno(e);
        }
        wasm32::__WASI_ESUCCESS
    } else {
        wasm32::__WASI_EOVERFLOW
    }
}

pub fn wasi_random_get(
    vmctx: &mut Vmctx,
    buf_ptr: wasm32::uintptr_t,
    buf_len: wasm32::size_t,
) -> wasm32::__wasi_errno_t {
    use rand::{thread_rng, RngCore};

    let buf = match dec_slice_of_mut::<u8>(vmctx, buf_ptr, buf_len) {
        Ok(buf) => buf,
        Err(e) => return enc_errno(e),
    };
    thread_rng().fill_bytes(buf);

    return wasm32::__WASI_ESUCCESS;
}

fn _wasi_poll_oneoff_handle_timeout_event(
    output_slice: &mut [wasm32::__wasi_event_t],
    timeout: Option<ClockEventData>,
) -> wasm32::size_t {
    if let Some(ClockEventData { userdata, .. }) = timeout {
        let output_event = host::__wasi_event_t {
            userdata,
            type_: wasm32::__WASI_EVENTTYPE_CLOCK,
            error: wasm32::__WASI_ESUCCESS,
            u: host::__wasi_event_t___wasi_event_u {
                fd_readwrite: host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                    nbytes: 0,
                    flags: 0,
                },
            },
        };
        output_slice[0] = enc_event(output_event);
        1
    } else {
        // shouldn't happen
        0
    }
}

fn _wasi_poll_oneoff_handle_fd_event<'t>(
    output_slice: &mut [wasm32::__wasi_event_t],
    events: impl Iterator<Item = (&'t FdEventData, &'t nix::poll::PollFd)>,
) -> wasm32::size_t {
    let mut output_slice_cur = output_slice.iter_mut();
    let mut revents_count = 0;
    for (fd_event, poll_fd) in events {
        let revents = match poll_fd.revents() {
            Some(revents) => revents,
            None => continue,
        };
        let mut nbytes = 0;
        if fd_event.type_ == wasm32::__WASI_EVENTTYPE_FD_READ {
            let _ = unsafe { fionread(fd_event.fd, &mut nbytes) };
        }
        let output_event = if revents.contains(nix::poll::EventFlags::POLLNVAL) {
            host::__wasi_event_t {
                userdata: fd_event.userdata,
                type_: fd_event.type_,
                error: wasm32::__WASI_EBADF,
                u: host::__wasi_event_t___wasi_event_u {
                    fd_readwrite:
                        host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                            nbytes: 0,
                            flags: wasm32::__WASI_EVENT_FD_READWRITE_HANGUP,
                        },
                },
            }
        } else if revents.contains(nix::poll::EventFlags::POLLERR) {
            host::__wasi_event_t {
                userdata: fd_event.userdata,
                type_: fd_event.type_,
                error: wasm32::__WASI_EIO,
                u: host::__wasi_event_t___wasi_event_u {
                    fd_readwrite:
                        host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                            nbytes: 0,
                            flags: wasm32::__WASI_EVENT_FD_READWRITE_HANGUP,
                        },
                },
            }
        } else if revents.contains(nix::poll::EventFlags::POLLHUP) {
            host::__wasi_event_t {
                userdata: fd_event.userdata,
                type_: fd_event.type_,
                error: wasm32::__WASI_ESUCCESS,
                u: host::__wasi_event_t___wasi_event_u {
                    fd_readwrite:
                        host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                            nbytes: 0,
                            flags: wasm32::__WASI_EVENT_FD_READWRITE_HANGUP,
                        },
                },
            }
        } else if revents.contains(nix::poll::EventFlags::POLLIN)
            | revents.contains(nix::poll::EventFlags::POLLOUT)
        {
            host::__wasi_event_t {
                userdata: fd_event.userdata,
                type_: fd_event.type_,
                error: wasm32::__WASI_ESUCCESS,
                u: host::__wasi_event_t___wasi_event_u {
                    fd_readwrite:
                        host::__wasi_event_t___wasi_event_u___wasi_event_u_fd_readwrite_t {
                            nbytes: nbytes as host::__wasi_filesize_t,
                            flags: 0,
                        },
                },
            }
        } else {
            continue;
        };
        *output_slice_cur.next().unwrap() = enc_event(output_event);
        revents_count += 1;
    }
    revents_count
}

pub fn wasi_poll_oneoff(
    vmctx: &mut Vmctx,
    input: wasm32::uintptr_t,
    output: wasm32::uintptr_t,
    nsubscriptions: wasm32::size_t,
    nevents: wasm32::uintptr_t,
) -> wasm32::__wasi_errno_t {
    if nsubscriptions as u64 > wasm32::__wasi_filesize_t::max_value() {
        return wasm32::__WASI_EINVAL;
    }
    enc_pointee(vmctx, nevents, 0).unwrap();

    let input_slice =
        dec_slice_of::<wasm32::__wasi_subscription_t>(vmctx, input, nsubscriptions).unwrap();

    let output_slice =
        dec_slice_of_mut::<wasm32::__wasi_event_t>(vmctx, output, nsubscriptions).unwrap();

    let input: Vec<_> = input_slice.iter().map(|x| dec_subscription(x)).collect();

    let timeout = input
        .iter()
        .filter_map(|event| match event {
            Ok(event) if event.type_ == wasm32::__WASI_EVENTTYPE_CLOCK => Some(ClockEventData {
                delay: wasi_clock_to_relative_ns_delay(unsafe { event.u.clock }) / 1_000_000,
                userdata: event.userdata,
            }),
            _ => None,
        })
        .min_by_key(|event| event.delay);
    let fd_events: Vec<_> = input
        .iter()
        .filter_map(|event| match event {
            Ok(event)
                if event.type_ == wasm32::__WASI_EVENTTYPE_FD_READ
                    || event.type_ == wasm32::__WASI_EVENTTYPE_FD_WRITE =>
            {
                Some(FdEventData {
                    fd: unsafe { event.u.fd_readwrite.fd } as c_int,
                    type_: event.type_,
                    userdata: event.userdata,
                })
            }
            _ => None,
        })
        .collect();
    if fd_events.is_empty() && timeout.is_none() {
        return wasm32::__WASI_ESUCCESS;
    }
    let mut poll_fds: Vec<_> = fd_events
        .iter()
        .map(|event| {
            let mut flags = nix::poll::EventFlags::empty();
            match event.type_ {
                wasm32::__WASI_EVENTTYPE_FD_READ => flags.insert(nix::poll::EventFlags::POLLIN),
                wasm32::__WASI_EVENTTYPE_FD_WRITE => flags.insert(nix::poll::EventFlags::POLLOUT),
                // An event on a file descriptor can currently only be of type FD_READ or FD_WRITE
                // Nothing else has been defined in the specification, and these are also the only two
                // events we filtered before. If we get something else here, the code has a serious bug.
                _ => unreachable!(),
            };
            nix::poll::PollFd::new(event.fd, flags)
        })
        .collect();
    let timeout = timeout.map(|ClockEventData { delay, userdata }| ClockEventData {
        delay: cmp::min(delay, c_int::max_value() as u128),
        userdata,
    });
    let poll_timeout = timeout.map(|timeout| timeout.delay as c_int).unwrap_or(-1);
    let ready = loop {
        match nix::poll::poll(&mut poll_fds, poll_timeout) {
            Err(_) => {
                if nix::errno::Errno::last() == nix::errno::Errno::EINTR {
                    continue;
                }
                return wasm32::errno_from_nix(nix::errno::Errno::last());
            }
            Ok(ready) => break ready as usize,
        }
    };
    let events_count = if ready == 0 {
        _wasi_poll_oneoff_handle_timeout_event(output_slice, timeout)
    } else {
        let events = fd_events.iter().zip(poll_fds.iter()).take(ready);
        _wasi_poll_oneoff_handle_fd_event(output_slice, events)
    };
    if let Err(e) = enc_pointee(vmctx, nevents, events_count) {
        return enc_errno(e);
    }
    wasm32::__WASI_ESUCCESS
}
