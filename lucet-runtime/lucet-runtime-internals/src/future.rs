use crate::instance::{InstanceHandle, InternalRunResult, RunResult, State, TerminationDetails};
use crate::module::FunctionHandle;
use crate::val::Val;
use crate::vmctx::{Vmctx, VmctxInternal};
use crate::{error::Error, instance::EmptyYieldVal};
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::{any::Any, future::Future};

/// a representation of AsyncContext which can be freely cloned
#[doc(hidden)]
#[derive(Clone)]
pub struct AsyncContext {
    waker: Waker,
}

const DEFAULT_INST_COUNT_BOUND: u64 = i64::MAX as u64;

/// A value representing that the guest instance yielded because it was blocked on a future.
struct AsyncYielded;

/// Providing the private `AsyncResume` as a resume value certifies that
/// RunAsync upheld the invariants necessary to safely resume the instance.
struct AsyncResume;

/// An error representing a failure of `try_block_on`
#[doc(hidden)]
pub enum BlockOnError {
    NeedsAsyncContext,
}

impl From<BlockOnError> for TerminationDetails {
    fn from(err: BlockOnError) -> Self {
        match err {
            BlockOnError::NeedsAsyncContext => TerminationDetails::BlockOnNeedsAsync,
        }
    }
}

impl Vmctx {
    /// Block on the result of an `async` computation from an instance run by `Instance::run_async`.
    ///
    /// While this method is supported and part of the public API, it's easiest to define the hostcall
    /// function itself as async (the `#[lucet_hostcall]` macro simply calls this function).
    /// If you need to provide a synchronous fallback, use [`Vmctx::try_block_on()`] instead.
    ///
    /// Note:
    /// - This method may only be used if `Instance::run_async` was used to run the VM,
    ///   otherwise it will terminate the instance with `TerminationDetails::BlockOnNeedsAsync`.
    #[doc(hidden)]
    #[inline(always)]
    pub fn block_on<R>(&self, f: impl Future<Output = R>) -> R {
        match self.try_block_on(f) {
            Ok(res) => res,
            Err(err) => panic!(TerminationDetails::from(err)),
        }
    }

    /// Block on the result of an `async` computation from an instance run by `Instance::run_async`.
    ///
    /// The easiest way to make a hostcall async is simply to add the `async` modifier:
    ///
    /// ```ignore
    /// #[lucet_hostcall]
    /// #[no_mangle]
    /// pub async fn hostcall_async(vmctx: &Vmctx) {
    ///    foobar().await
    /// }
    /// ```
    ///
    /// Of course, we can only expose synchronous interfaces to Wasm guests. To implement
    /// async hostcalls, Lucet blocks guest execution while waiting the future to complete,
    /// yielding control of the thread back the async executor so that other tasks can make
    /// progress in the meantime.
    ///
    /// It's recommended to use the async modifier, rather than call this function directly.
    /// The primary reason you may want to do so directly is if you want to provide a fallback
    /// implementation for the case when calling a hostcall from outside of an async context.
    ///
    /// There is no additional yield to the host when the future passed is immediately ready.
    /// This behavior is the same as implemented by the `.await` operator.
    ///
    /// The future passed is polled from within the guest execution context. If the future is not
    /// immediately ready, the instance will yield and return control to the async executor.
    /// Later, once the future is ready to make progress, the async executor will return us to
    /// the guest context, where Lucet will resume polling the future to completion.
    ///
    /// For async hostcalls that may yield to the async executor many times, it's recommended that
    /// you use `tokio::spawn`, or the equivalent from your async executor, which will spawn the task
    /// to be run from the host execution context. This avoids the overhead of context switching into
    /// the guest execution context every time the future needs to make progress.
    ///
    /// If `Instance::run_async` is not being used to run the VM, this function will return
    /// `Err(BlockOnError::NeedsAsyncContext)`.
    pub fn try_block_on<R>(&self, mut f: impl Future<Output = R>) -> Result<R, BlockOnError> {
        // We pin the future to the stack (a requirement for being able to poll the future).
        // By pinning to the stack instead of using `Box::pin`, we avoid heap allocations for
        // immediately-ready futures.
        //
        // SAFETY: We must uphold the invariants of Pin, namely that future does not move until
        // it is dropped. By overriding the variable named `f`, it is impossible to access f again,
        // except through the pinned reference.
        let mut f = unsafe { Pin::new_unchecked(&mut f) };

        loop {
            // Get the AsyncContext, or die if we aren't async
            let arc_cx = match &self.instance().state {
                State::Running {
                    async_context: Some(cx),
                } => cx.clone(),
                State::Running {
                    async_context: None,
                } => return Err(BlockOnError::NeedsAsyncContext),
                _ => unreachable!("Access to vmctx implies instance is Running"),
            };

            // build an std::task::Context
            let mut cx = Context::from_waker(&arc_cx.waker);

            if let Poll::Ready(ret) = f.as_mut().poll(&mut cx) {
                return Ok(ret);
            }

            // The future is pending, so we need to yield to the async executor
            self.yield_impl::<AsyncYielded, AsyncResume>(AsyncYielded, false, false);

            // Poll is synchronous and may have yielded back to this host, so it is possible that we are
            // executing from a new RunAsync future (and thus, a new async executor). If the future has awoken
            // the previous waker, we would miss a wakeup.
            //
            // Rather than try to prevent this from happening at all, we check if the waker changed from under us,
            // and if so, we simply poll the future again. If the future is Ready by that point, there's no need
            // for another wakeup (we've already done so!). Otherwise, polling the future registers the new waker,
            // allowing us to yield back to the async executor without risking a missed wakup.
            //
            // Note: Waking a waker unnecessarily will not cause unsafety or logic errors. In the worst case, the
            // async executor may waste CPU time polling pending futures an extra time.

            match &self.instance().state {
                State::Running { async_context: Some(new_cx) } => {
                    let same_waker = Arc::ptr_eq(&arc_cx, &new_cx) || arc_cx.waker.will_wake(&new_cx.waker);

                    if !same_waker {
                        // The AsyncContext changed on us.
                        // Poll the future again before yielding to the executor in order to register the new waker.
                        continue;
                    }
                },
                _ => panic!("Lucet instance blocked on a future, but no longer running in async context. Make sure to use resume_async when resuming an async guest.")
            }

            // Providing the private `AsyncResume` as a resume value certifies that
            // RunAsync upheld the invariants necessary for us to avoid a borrow check.
            //
            // If we resume with any other value, the instance may have been modified, and it is
            // unsound to resume the instance.
            let AsyncResume = self.take_resumed_val::<AsyncResume>();
        }
    }
}

impl InstanceHandle {
    /// Run a WebAssembly function with arguments in the guest context at the given entrypoint.
    ///
    /// This method is similar to `Instance::run()`, but allows the Wasm program to invoke async hostcalls.
    ///
    /// To define an async hostcall, simply add an `async` modifier to your hostcall:
    ///
    /// ```ignore
    /// #[lucet_hostcall]
    /// #[no_mangle]
    /// pub async fn hostcall_async(vmctx: &Vmctx) {
    ///    foobar().await
    /// }
    /// ```
    ///
    /// See [`Vmctx::try_block_on()`] for details.
    ///
    /// If `runtime_bound` is provided, it will also pause the Wasm execution and yield a future
    /// that resumes it after (approximately) that many Wasm opcodes have executed.
    pub fn run_async<'a>(&'a mut self, entrypoint: &'a str, args: &'a [Val]) -> RunAsync<'a> {
        let func = self.module.get_export_func(entrypoint);

        match func {
            Ok(func) => self.run_async_internal(func, args),
            Err(err) => self.run_async_failed(err),
        }
    }

    /// Run the module's [start function][start], if one exists.
    ///
    /// If there is no start function in the module, this does nothing.
    ///
    /// All of the other restrictions on the start function, what it may do, and
    /// the requirement that it must be invoked first, are described in the
    /// documentation for `Instance::run_start()`. This async version of that
    /// function satisfies the requirement to run the start function first, as
    /// long as the async function fully returns (not just yields).
    ///
    /// This method is similar to `Instance::run_start()`, except that it bounds
    /// runtime between async future yields (invocations of `.poll()` on the
    /// underlying generated future) if `runtime_bound` is provided. This
    /// behaves the same way as `Instance::run_async()`.
    ///
    /// Just like `Instance::run_start()`, hostcalls, including async hostcalls,
    /// cannot be called from the instance start function.
    ///
    /// The result of the `RunAsync` future is unspecified, and should not be relied on.
    pub fn run_async_start<'a>(&'a mut self) -> RunAsync<'a> {
        if !self.is_not_started() {
            return self.run_async_failed(Error::StartAlreadyRun);
        }

        let func = match self.module.get_start_func() {
            Ok(start) => start.expect("NotStarted, but no start function"), // can only be in NotStarted state if a start function exists,
            Err(err) => return self.run_async_failed(err),
        };

        self.run_async_internal(func, &[])
    }

    /// Resume async execution of an instance that has yielded, providing a value to the guest.
    ///
    /// If an async execution context yields from within a future, resuming with [`Instance::resume()`],
    /// [`Instance::resume_with_val()`], may panic if the instance needs to block on an async function.
    /// Use this function instead, which will resume the instance within an async context.
    ///
    /// The provided value will be dynamically typechecked against the type the guest expects to
    /// receive, and if that check fails, this call will fail with `Error::InvalidArgument`.
    ///
    /// See [`Instance::resume()`], [`Instance::resume_with_val()`], and [`Instance::run_async()`].
    ///
    /// # Safety
    ///
    /// The foreign code safety caveat of [`Instance::run()`](struct.Instance.html#method.run)
    /// applies.
    pub fn resume_async_with_val<'a>(&'a mut self, val: impl Any + 'static + Send) -> RunAsync<'a> {
        let val = Box::new(val) as Box<dyn Any + 'static + Send>;

        RunAsync {
            inst: self,
            inst_count_bound: DEFAULT_INST_COUNT_BOUND,
            state: RunAsyncState::ResumeYielded(val),
        }
    }

    /// Resume execution of an instance that has yielded without providing a value to the guest.
    ///
    /// See [`Instance::resume_async_with_val()`]
    pub fn resume_async<'a>(&'a mut self) -> RunAsync<'a> {
        self.resume_async_with_val(EmptyYieldVal)
    }

    /// Returns a `RunAsync` that will asynchronously execute the guest instnace.
    fn run_async_internal<'a>(&'a mut self, func: FunctionHandle, args: &'a [Val]) -> RunAsync<'a> {
        RunAsync {
            inst: self,
            inst_count_bound: DEFAULT_INST_COUNT_BOUND,
            state: RunAsyncState::Start(func, args),
        }
    }

    /// Returns a `RunAsync` that will immediately fail with the given error, without executing the guest instance.
    fn run_async_failed<'a>(&'a mut self, err: Error) -> RunAsync<'a> {
        RunAsync {
            inst: self,
            inst_count_bound: DEFAULT_INST_COUNT_BOUND,
            state: RunAsyncState::Failed(err),
        }
    }
}

/// A Future implementation that enables asynchronous execution of bounded slices of
/// WebAssembly, and of underlying async hostcalls that it invokes.
///
/// See [`Vmctx::try_block_on()`] for more details about how this works, and how
/// to define an async hostcall.
pub struct RunAsync<'a> {
    inst: &'a mut InstanceHandle,
    state: RunAsyncState<'a>,
    /// The instance count bound. Can be changed at any time, taking effect on the next entry to the guest execution context
    pub inst_count_bound: u64,
}

impl<'a> RunAsync<'a> {
    /// Set the instance count bound
    pub fn bound_inst_count(&mut self, inst_count_bound: u64) -> &mut Self {
        self.inst_count_bound = inst_count_bound;
        self
    }
}

enum RunAsyncState<'a> {
    Start(FunctionHandle, &'a [Val]),
    ResumeYielded(Box<dyn Any + 'static + Send>),
    BoundExpired,
    Failed(Error),
}

impl<'a> Future for RunAsync<'a> {
    type Output = Result<RunResult, Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inst_count_bound = self.inst_count_bound;

        let waker = cx.waker();
        let cx = AsyncContext {
            waker: waker.clone(),
        };

        let state = std::mem::replace(
            &mut self.state,
            RunAsyncState::Failed(Error::InvalidArgument("Polled an invalid future")),
        );
        let run_result = match state {
            RunAsyncState::Start(func, args) => {
                // This is the first iteration, call the entrypoint:
                self.inst
                    .run_func(func, args, Some(cx), Some(inst_count_bound))
            }
            RunAsyncState::ResumeYielded(val) => {
                self.inst
                    .resume_with_val_impl(val, Some(cx), Some(inst_count_bound))
            }
            RunAsyncState::BoundExpired => self.inst.resume_bounded(cx, inst_count_bound),
            RunAsyncState::Failed(err) => Err(err),
        };

        let res = match run_result {
            Ok(InternalRunResult::Normal(r @ RunResult::Returned(_))) => Ok(r),
            Ok(InternalRunResult::Normal(RunResult::Yielded(yval))) => {
                match yval.downcast::<AsyncYielded>() {
                    Ok(_) => {
                        // When this future is polled next, we'll resume the guest instance using `AsyncResume`
                        self.state = RunAsyncState::ResumeYielded(Box::new(AsyncResume));
                        return Poll::Pending;
                    }
                    Err(yval) => Ok(RunResult::Yielded(yval)),
                }
            }
            Ok(InternalRunResult::BoundExpired) => {
                // The instruction count bound expired. Yield to the async exeuctor and immediately wake.
                //
                // By immediately waking, the future will be scheduled to run later (similar to tokio's yield_now())
                self.state = RunAsyncState::BoundExpired;
                waker.wake_by_ref();
                return Poll::Pending;
            }
            Err(err) => Err(err),
        };

        Poll::Ready(res)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Show that the futures returned by `InstanceHandle` methods are `Send`.
    ///
    /// This doesn't actually do anything at runtime, the fact that it compiles is what counts.
    #[test]
    fn async_futures_are_send() {
        fn _assert_send<T: Send>(_: &T) {}
        #[allow(unreachable_code)]
        fn _dont_run_me() {
            let mut _inst: InstanceHandle = unimplemented!();
            _assert_send(&_inst.run_async("", &[]));
            _assert_send(&_inst.run_async_start());
        }
    }
}
