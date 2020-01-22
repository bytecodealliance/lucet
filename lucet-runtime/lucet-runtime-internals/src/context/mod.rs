#![allow(improper_ctypes)]

#[cfg(test)]
mod tests;

use crate::instance::Instance;
use crate::val::{val_to_reg, val_to_stack, RegVal, UntypedRetVal, Val};
use nix;
use nix::sys::signal;
use std::arch::x86_64::{__m128, _mm_setzero_ps};
use std::ptr::NonNull;
use std::{mem, ptr};
use thiserror::Error;

/// Callee-saved general-purpose registers in the AMD64 ABI.
///
/// # Layout
///
/// `repr(C)` is required to preserve the ordering of members, which are read by the assembly at
/// hard-coded offsets.
///
/// # TODOs
///
/// - Unlike the C code, this doesn't use the `packed` repr due to warnings in the Nomicon:
/// <https://doc.rust-lang.org/nomicon/other-reprs.html#reprpacked>. Since the members are all
/// `u64`, this should be fine?
#[repr(C)]
pub(crate) struct GpRegs {
    rbx: u64,
    pub(crate) rsp: u64,
    rbp: u64,
    pub(crate) rdi: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    pub(crate) rsi: u64,
}

impl GpRegs {
    fn new() -> Self {
        GpRegs {
            rbx: 0,
            rsp: 0,
            rbp: 0,
            rdi: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rsi: 0,
        }
    }
}

/// Floating-point argument registers in the AMD64 ABI.
///
/// # Layout
///
/// `repr(C)` is required to preserve the ordering of members, which are read by the assembly at
/// hard-coded offsets.
///
/// # TODOs
///
/// - Unlike the C code, this doesn't use the `packed` repr due to warnings in the Nomicon:
/// <https://doc.rust-lang.org/nomicon/other-reprs.html#reprpacked>. Since the members are all
/// `__m128`, this should be fine?
#[repr(C)]
struct FpRegs {
    xmm0: __m128,
    xmm1: __m128,
    xmm2: __m128,
    xmm3: __m128,
    xmm4: __m128,
    xmm5: __m128,
    xmm6: __m128,
    xmm7: __m128,
}

impl FpRegs {
    fn new() -> Self {
        let zero = unsafe { _mm_setzero_ps() };
        FpRegs {
            xmm0: zero,
            xmm1: zero,
            xmm2: zero,
            xmm3: zero,
            xmm4: zero,
            xmm5: zero,
            xmm6: zero,
            xmm7: zero,
        }
    }
}

/// Everything we need to make a context switch: a signal mask, and the registers and return values
/// that are manipulated directly by assembly code.
///
/// A context also tracks which other context to swap back to if a child context's entrypoint function
/// returns, and can optionally contain a callback function to be run just before that swap occurs.
///
/// # Layout
///
/// The `repr(C)` and order of fields in this struct are very important, as the assembly code reads
/// and writes hard-coded offsets from the base of the struct. Without `repr(C)`, Rust is free to
/// reorder the fields.
///
/// Contexts are also `repr(align(64))` in order to align to cache lines and minimize contention
/// when running multiple threads.
///
/// # Movement
///
/// `Context` values must not be moved once they've been initialized. Contexts contain a pointer to
/// their stack, which in turn contains a pointer back to the context. If the context gets moved,
/// that pointer becomes invalid, and the behavior of returning from that context becomes undefined.
#[repr(C, align(64))]
pub struct Context {
    pub(crate) gpr: GpRegs,
    fpr: FpRegs,
    retvals_gp: [u64; 2],
    retval_fp: __m128,
    parent_ctx: *mut Context,
    // TODO ACF 2019-10-23: make Instance into a generic parameter?
    backstop_callback: *const unsafe extern "C" fn(*mut Instance),
    backstop_data: *mut Instance,
    sigset: signal::SigSet,
}

impl Context {
    /// Create an all-zeroed `Context`.
    pub fn new() -> Self {
        Context {
            gpr: GpRegs::new(),
            fpr: FpRegs::new(),
            retvals_gp: [0; 2],
            retval_fp: unsafe { _mm_setzero_ps() },
            parent_ctx: ptr::null_mut(),
            backstop_callback: Context::default_backstop_callback as *const _,
            backstop_data: ptr::null_mut(),
            sigset: signal::SigSet::empty(),
        }
    }
}

/// A wrapper around a `Context`, primarily meant for use in test code.
///
/// Users of this library interact with contexts implicitly via `Instance` values, but for testing
/// the context code independently, it is helpful to use contexts directly.
///
/// # Movement of `ContextHandle`
///
/// `ContextHandle` keeps a pointer to a `Context` rather than keeping all of the data directly as
/// fields in order to have better control over where that data lives in memory. We always want that
/// data to be heap-allocated, and to never move once it has been initialized. The `ContextHandle`,
/// by contrast, should be treated like a normal Rust value with no such restrictions.
///
/// Until the `Unpin` marker trait arrives in stable Rust, it is difficult to enforce this with the
/// type system alone, so we use a bit of unsafety and (hopefully) clever API design to ensure that
/// the data cannot be moved.
///
/// We create the `Context` within a box to allocate it on the heap, then convert it into a raw
/// pointer to relinquish ownership. When accessing the internal structure via the `DerefMut` trait,
/// data must not be moved out of the `Context` with functions like `mem::replace`.
///
/// # Layout
///
/// Foreign code accesses the `internal` pointer in tests, so it is important that it is the first
/// member, and that the struct is `repr(C)`.
#[repr(C)]
pub struct ContextHandle {
    internal: NonNull<Context>,
}

impl Drop for ContextHandle {
    fn drop(&mut self) {
        unsafe {
            // create a box from the pointer so that it'll get dropped
            // and we won't leak `Context`s
            Box::from_raw(self.internal.as_ptr());
        }
    }
}

impl std::ops::Deref for ContextHandle {
    type Target = Context;
    fn deref(&self) -> &Self::Target {
        unsafe { self.internal.as_ref() }
    }
}

impl std::ops::DerefMut for ContextHandle {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.internal.as_mut() }
    }
}

impl ContextHandle {
    /// Create an all-zeroed `ContextHandle`.
    pub fn new() -> Self {
        let internal = NonNull::new(Box::into_raw(Box::new(Context::new())))
            .expect("Box::into_raw should never return NULL");
        ContextHandle { internal }
    }

    pub fn create_and_init(
        stack: &mut [u64],
        fptr: usize,
        args: &[Val],
    ) -> Result<ContextHandle, Error> {
        let mut child = ContextHandle::new();
        Context::init(stack, &mut child, fptr, args)?;
        Ok(child)
    }
}

struct CallStackBuilder<'a> {
    offset: usize,
    stack: &'a mut [u64],
}

impl<'a> CallStackBuilder<'a> {
    pub fn new(stack: &'a mut [u64]) -> Self {
        CallStackBuilder { offset: 0, stack }
    }

    fn push(&mut self, val: u64) {
        self.offset += 1;
        self.stack[self.stack.len() - self.offset] = val;
    }

    /// Stores `args` onto the stack such that when a return address is written after, the
    /// complete unit will be 16-byte aligned, as the x86_64 ABI requires.
    ///
    /// That is to say, `args` will be padded such that the current top of stack is 8-byte
    /// aligned.
    fn store_args(&mut self, args: &[u64]) {
        let items_end = args.len() + self.offset;

        if items_end % 2 == 1 {
            // we need to add one entry just before the arguments so that the arguments start on an
            // aligned address.
            self.push(0);
        }

        for arg in args.iter().rev() {
            self.push(*arg);
        }
    }

    fn offset(&self) -> usize {
        self.offset
    }

    fn into_inner(self) -> (&'a mut [u64], usize) {
        (self.stack, self.offset)
    }
}

impl Context {
    /// Initialize a new child context.
    ///
    /// - `stack`: The stack for the child; *must be 16-byte aligned*.
    ///
    /// - `child`: The context for the child. The fields of this structure will be overwritten by
    /// `init`.
    ///
    /// - `fptr`: A pointer to the entrypoint for the child. Note that while the type signature here
    /// is for a void function of no arguments (equivalent to `void (*fptr)(void)` in C), the
    /// entrypoint actually can be a function of any argument or return type that corresponds to a
    /// `val::Val` variant.
    ///
    /// - `args`: A slice of arguments for the `fptr` entrypoint. These must match the number and
    /// types of `fptr`'s actual arguments exactly, otherwise swapping to this context will cause
    /// undefined behavior.
    ///
    /// # Errors
    ///
    /// - `Error::UnalignedStack` if the _end_ of `stack` is not 16-byte aligned.
    ///
    /// # Examples
    ///
    /// ## C entrypoint
    ///
    /// This example initializes a context that will start in a C function `entrypoint` when first
    /// swapped to.
    ///
    /// ```c
    /// void entrypoint(uint64_t x, float y);
    /// ```
    ///
    /// ```no_run
    /// # use lucet_runtime_internals::context::Context;
    /// # use lucet_runtime_internals::val::Val;
    /// extern "C" { fn entrypoint(x: u64, y: f32); }
    /// // allocating an even number of `u64`s seems to reliably yield
    /// // properly aligned stacks, but TODO do better
    /// let mut stack = vec![0u64; 1024].into_boxed_slice();
    /// let mut child = Context::new();
    /// let res = Context::init(
    ///     &mut *stack,
    ///     &mut child,
    ///     entrypoint as usize,
    ///     &[Val::U64(120), Val::F32(3.14)],
    /// );
    /// assert!(res.is_ok());
    /// ```
    ///
    /// ## Rust entrypoint
    ///
    /// This example initializes a context that will start in a Rust function `entrypoint` when
    /// first swapped to. Note that we mark `entrypoint` as `extern "C"` to make sure it is compiled
    /// with C calling conventions.
    ///
    /// ```no_run
    /// # use lucet_runtime_internals::context::{Context, ContextHandle};
    /// # use lucet_runtime_internals::val::Val;
    /// extern "C" fn entrypoint(x: u64, y: f32) { }
    /// // allocating an even number of `u64`s seems to reliably yield
    /// // properly aligned stacks, but TODO do better
    /// let mut stack = vec![0u64; 1024].into_boxed_slice();
    /// let mut child = Context::new();
    /// let res = Context::init(
    ///     &mut *stack,
    ///     &mut child,
    ///     entrypoint as usize,
    ///     &[Val::U64(120), Val::F32(3.14)],
    /// );
    /// assert!(res.is_ok());
    /// ```
    ///
    /// # Implementation details
    ///
    /// This prepares a stack for the child context structured as follows, assuming an 0x1000 byte
    /// stack:
    /// ```text
    /// 0x1000: +-------------------------+
    /// 0x0ff8: | NULL                    | // Null added if necessary for alignment.
    /// 0x0ff0: | spilled_arg_1           | // Guest arguments follow.
    /// 0x0fe8: | spilled_arg_2           |
    /// 0x0fe0: ~ spilled_arg_3           ~ // The three arguments here are just for show.
    /// 0x0fd8: | lucet_context_backstop  | <-- This forms an ABI-matching call frame for fptr.
    /// 0x0fd0: | fptr                    | <-- The actual guest code we want to run.
    /// 0x0fc8: | lucet_context_bootstrap | <-- The guest stack pointer starts here.
    /// 0x0fc0: |                         |
    /// 0x0XXX: ~                         ~ // Rest of the stack needs no preparation.
    /// 0x0000: |                         |
    ///         +-------------------------+
    /// ```
    ///
    /// This packing of data on the stack is interwoven with noteworthy constraints on what the
    /// backstop may do:
    /// * The backstop must not return on the guest stack.
    ///   - The next value will be a spilled argument or NULL. Neither are an intended address.
    /// * The backstop cannot have ABI-conforming spilled arguments.
    ///   - No code runs between `fptr` and `lucet_context_backstop`, so nothing exists to
    ///     clean up `fptr`'s arguments. `lucet_context_backstop` would have to adjust the
    ///     stack pointer by a variable amount, and it does not, so `rsp` will continue to
    ///     point to guest arguments.
    ///   - This is why bootstrap recieves arguments via rbp, pointing elsewhere on the stack.
    ///
    /// The bootstrap function must be careful, but is less constrained since it can clean up
    /// and prepare a context for `fptr`.
    pub fn init(
        stack: &mut [u64],
        child: &mut Context,
        fptr: usize,
        args: &[Val],
    ) -> Result<(), Error> {
        Context::init_with_callback(
            stack,
            child,
            Context::default_backstop_callback,
            ptr::null_mut(),
            fptr,
            args,
        )
    }

    /// The default backstop callback does nothing, and is just a marker.
    extern "C" fn default_backstop_callback(_: *mut Instance) {}

    /// Similar to `Context::init()`, but allows setting a callback function to be run when the
    /// guest entrypoint returns.
    ///
    /// After the entrypoint function returns, but before swapping back to the parent context,
    /// `backstop_callback` will be run with the single argument `backstop_data`.
    pub fn init_with_callback(
        stack: &mut [u64],
        child: &mut Context,
        backstop_callback: unsafe extern "C" fn(*mut Instance),
        backstop_data: *mut Instance,
        fptr: usize,
        args: &[Val],
    ) -> Result<(), Error> {
        if !stack_is_aligned(stack) {
            return Err(Error::UnalignedStack);
        }

        if backstop_callback != Context::default_backstop_callback {
            child.backstop_callback = backstop_callback as *const _;
            child.backstop_data = backstop_data;
        }

        let mut gp_args_ix = 0;
        let mut fp_args_ix = 0;
        let mut gp_regs_values = [0u64; 6];

        let mut spilled_args = vec![];

        for arg in args {
            match val_to_reg(arg) {
                RegVal::GpReg(v) => {
                    if gp_args_ix >= 6 {
                        spilled_args.push(val_to_stack(arg));
                    } else {
                        gp_regs_values[gp_args_ix] = v;
                        gp_args_ix += 1;
                    }
                }
                RegVal::FpReg(v) => {
                    if fp_args_ix >= 8 {
                        spilled_args.push(val_to_stack(arg));
                    } else {
                        child.bootstrap_fp_ix_arg(fp_args_ix, v);
                        fp_args_ix += 1;
                    }
                }
            }
        }

        // set up an initial call stack for guests to bootstrap into and execute
        let mut stack_builder = CallStackBuilder::new(stack);

        // we actually don't want to put an explicit pointer to these arguments anywhere. we'll
        // line up the rest of the stack such that these are in argument position when we jump to
        // `fptr`.
        stack_builder.store_args(spilled_args.as_slice());

        // the stack must be aligned in the environment we'll execute `fptr` from - this is an ABI
        // requirement and can cause segfaults if not upheld.
        assert_eq!(
            stack_builder.offset() % 2,
            0,
            "incorrect alignment for guest call frame"
        );

        // we execute the guest code via returns, so we make a "call stack" of routines like:
        // -> lucet_context_backstop()
        //    -> fptr()
        //       -> lucet_context_bootstrap()
        //
        // with each address the start of the named function, so when the inner function
        // completes it returns to begin the next function up.
        stack_builder.push(lucet_context_backstop as u64);
        stack_builder.push(fptr as u64);

        // add all general purpose arguments for the guest to be bootstrapped
        for arg in gp_regs_values.iter() {
            stack_builder.push(*arg);
        }

        stack_builder.push(lucet_context_bootstrap as u64);

        let (stack, stack_start) = stack_builder.into_inner();

        // RSP, RBP, and sigset still remain to be initialized.
        // Stack pointer: this points to the return address that will be used by `swap`, in place
        // of the original (eg, in the host) return address. The return address this points to is
        // the address of the first function to run on `swap`: `lucet_context_bootstrap`.
        child.gpr.rsp = &mut stack[stack.len() - stack_start] as *mut u64 as u64;

        child.gpr.rbp = child as *const Context as u64;

        // Read the mask to be restored if we ever need to jump out of a signal handler. If this
        // isn't possible, die.
        signal::pthread_sigmask(
            signal::SigmaskHow::SIG_SETMASK,
            None,
            Some(&mut child.sigset),
        )
        .expect("pthread_sigmask could not be retrieved");

        Ok(())
    }

    /// Save the current context, and swap to another context.
    ///
    /// - `from`: the current context is written here
    /// - `to`: the context to read from and swap to
    ///
    /// The current registers, including the stack pointer, are saved to `from`. The current stack
    /// pointer is then replaced by the value saved in `to.gpr.rsp`, so when `swap` returns, it will
    /// return to the pointer saved in `to`'s stack.
    ///
    /// If `to` was freshly initialized by passing it as the `child` argument to `init`, `swap` will
    /// return to the function that bootstraps arguments and then calls the entrypoint that was
    /// passed to `init`.
    ///
    /// If `to` was previously passed as the `from` argument to another call to `swap`, the program
    /// will return as if from that _first_ call to `swap`.
    ///
    /// The address of `from` will be saved as `to.parent_ctx`. If `to` was initialized by `init`,
    /// it will swap back to the `from` context when the entrypoint function returns via
    /// `lucet_context_backstop`.
    ///
    /// # Safety
    ///
    /// The value in `to.gpr.rsp` must be a valid pointer into the stack that was originally passed
    /// to `init` when the `to` context was initialized, or to the original stack created implicitly
    /// by Rust.
    ///
    /// The registers saved in the `to` context must match the arguments expected by the entrypoint
    /// of the function passed to `init`, or be unaltered from when they were previously written by
    /// `swap`.
    ///
    /// If `to` was initialized by `init`, the `from` context must not be moved, dropped, or
    /// otherwise invalidated while in the `to` context unless `to`'s entrypoint function never
    /// returns.
    ///
    /// If `from` is never returned to, `swap`ped to, or `set` to, resources could leak due to
    /// implicit `drop`s never being called:
    ///
    /// ```no_run
    /// # use lucet_runtime_internals::context::Context;
    /// fn f(x: Box<u64>, child: &mut Context) {
    ///     let mut xs = vec![187; 410757864530];
    ///     xs[0] += *x;
    ///
    ///     // manually drop here to avoid leaks
    ///     drop(x);
    ///     drop(xs);
    ///
    ///     let mut parent = Context::new();
    ///     unsafe { Context::swap(&mut parent, child); }
    ///     // implicit `drop(x)` and `drop(xs)` here never get called unless we swap back
    /// }
    /// ```
    ///
    /// # Examples
    ///
    /// The typical case is to initialize a new child context, and then swap to it from a zeroed
    /// parent context.
    ///
    /// ```no_run
    /// # use lucet_runtime_internals::context::Context;
    /// # extern "C" fn entrypoint() {}
    /// # let mut stack = vec![0u64; 1024].into_boxed_slice();
    /// let mut parent = Context::new();
    /// let mut child = Context::new();
    /// Context::init(
    ///     &mut stack,
    ///     &mut child,
    ///     entrypoint as usize,
    ///     &[],
    /// ).unwrap();
    ///
    /// unsafe { Context::swap(&mut parent, &mut child); }
    /// ```
    #[inline]
    pub unsafe fn swap(from: &mut Context, to: &mut Context) {
        to.parent_ctx = from;
        lucet_context_swap(from as *mut _, to as *mut _);
    }

    /// Swap to another context without saving the current context.
    ///
    /// - `to`: the context to read from and swap to
    ///
    /// The current registers, including the stack pointer, are discarded. The current stack pointer
    /// is then replaced by the value saved in `to.gpr.rsp`, so when `swap` returns, it will return
    /// to the pointer saved in `to`'s stack.
    ///
    /// If `to` was freshly initialized by passing it as the child to `init`, `swap` will return to
    /// the function that bootstraps arguments and then calls the entrypoint that was passed to
    /// `init`.
    ///
    /// If `to` was previously passed as the `from` argument to another call to `swap`, the program
    /// will return as if from the call to `swap`.
    ///
    /// # Safety
    ///
    /// ## Stack and registers
    ///
    /// The value in `to.gpr.rsp` must be a valid pointer into the stack that was originally passed
    /// to `init` when the context was initialized, or to the original stack created implicitly by
    /// Rust.
    ///
    /// The registers saved in `to` must match the arguments expected by the entrypoint of the
    /// function passed to `init`, or be unaltered from when they were previously written by `swap`.
    ///
    /// ## Returning
    ///
    /// If `to` is a context freshly initialized by `init` (as opposed to a context populated only
    /// by `swap`, such as a host context), at least one of the following must be true, otherwise
    /// the program will return to a context with uninitialized registers:
    ///
    /// - The `fptr` argument to `init` is a function that never returns
    ///
    /// - A valid context must have been passed as the `from` argument to `swap` when entering the
    ///   current context before this call to `set`
    ///
    /// ## Resource leaks
    ///
    /// Since control flow will not return to the calling context, care must be taken to ensure that
    /// any resources owned by the calling context are manually dropped. The implicit `drop`s
    /// inserted by Rust at the end of the calling scope will not be reached:
    ///
    /// ```no_run
    /// # use lucet_runtime_internals::context::Context;
    /// fn f(x: Box<u64>, child: &Context) {
    ///     let mut xs = vec![187; 410757864530];
    ///     xs[0] += *x;
    ///
    ///     // manually drop here to avoid leaks
    ///     drop(x);
    ///     drop(xs);
    ///
    ///     unsafe { Context::set(child); }
    ///     // implicit `drop(x)` and `drop(xs)` here never get called
    /// }
    /// ```
    #[inline]
    pub unsafe fn set(to: &Context) -> ! {
        lucet_context_set(to as *const Context);
    }

    /// Like `set`, but also manages the return from a signal handler.
    ///
    /// TODO: the return type of this function should really be `Result<!, nix::Error>`, but using
    /// `!` as a type like that is currently experimental.
    #[inline]
    pub unsafe fn set_from_signal(to: &Context) -> Result<(), nix::Error> {
        signal::pthread_sigmask(signal::SigmaskHow::SIG_SETMASK, Some(&to.sigset), None)?;
        Context::set(to)
    }

    /// Clear (zero) return values.
    pub fn clear_retvals(&mut self) {
        self.retvals_gp = [0; 2];
        let zero = unsafe { _mm_setzero_ps() };
        self.retval_fp = zero;
    }

    /// Get the general-purpose return value at index `idx`.
    ///
    /// If this method is called before the context has returned from its original entrypoint, the
    /// result will be `0`.
    pub fn get_retval_gp(&self, idx: usize) -> u64 {
        self.retvals_gp[idx]
    }

    /// Get the floating point return value.
    ///
    /// If this method is called before the context has returned from its original entrypoint, the
    /// result will be `0.0`.
    pub fn get_retval_fp(&self) -> __m128 {
        self.retval_fp
    }

    /// Get the return value as an `UntypedRetVal`.
    ///
    /// This combines the 0th general-purpose return value, and the single floating-point return value.
    pub fn get_untyped_retval(&self) -> UntypedRetVal {
        let gp = self.get_retval_gp(0);
        let fp = self.get_retval_fp();
        UntypedRetVal::new(gp, fp)
    }

    /// Put one of the first 8 floating-point arguments into a `Context` register.
    ///
    /// - `ix`: ABI floating-point argument number
    /// - `arg`: argument value
    fn bootstrap_fp_ix_arg(&mut self, ix: usize, arg: __m128) {
        match ix {
            0 => self.fpr.xmm0 = arg,
            1 => self.fpr.xmm1 = arg,
            2 => self.fpr.xmm2 = arg,
            3 => self.fpr.xmm3 = arg,
            4 => self.fpr.xmm4 = arg,
            5 => self.fpr.xmm5 = arg,
            6 => self.fpr.xmm6 = arg,
            7 => self.fpr.xmm7 = arg,
            _ => panic!("unexpected fp register index {}", ix),
        }
    }
}

/// Errors that may arise when working with contexts.
#[derive(Debug, Error)]
pub enum Error {
    /// Raised when the bottom of the stack provided to `Context::init` is not 16-byte aligned
    #[error("context initialized with unaligned stack")]
    UnalignedStack,
}

/// Check whether the bottom (highest address) of the stack is 16-byte aligned, as required by the
/// ABI.
fn stack_is_aligned(stack: &[u64]) -> bool {
    let size = stack.len();
    let last_elt_addr = &stack[size - 1] as *const u64 as usize;
    let bottom_addr = last_elt_addr + mem::size_of::<u64>();
    bottom_addr % 16 == 0
}

extern "C" {
    /// Bootstraps arguments and calls the entrypoint via returning; implemented in assembly.
    ///
    /// Loads general-purpose arguments from the callee-saved registers in a `Context` to the
    /// appropriate argument registers for the AMD64 ABI, and then returns to the entrypoint.
    fn lucet_context_bootstrap();

    /// Stores return values into the parent context, and then swaps to it; implemented in assembly.
    ///
    /// This is where the entrypoint function returns to, so that we swap back to the parent on
    /// return.
    fn lucet_context_backstop();

    /// Saves the current context and performs the context switch. Implemented in assembly.
    fn lucet_context_swap(from: *mut Context, to: *mut Context);

    /// Performs the context switch; implemented in assembly.
    ///
    /// Never returns because the current context is discarded.
    fn lucet_context_set(to: *const Context) -> !;

    /// Enables termination for the instance, after performing a context switch.
    ///
    /// Takes the guest return address as an argument as a consequence of implementation details,
    /// see `Instance::swap_and_return` for more.
    pub(crate) fn lucet_context_activate();
}
