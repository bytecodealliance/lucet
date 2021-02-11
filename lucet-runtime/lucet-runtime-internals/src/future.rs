use crate::error::Error;
use crate::instance::{InstanceHandle, InternalRunResult, RunResult, State, TerminationDetails};
use crate::module::FunctionHandle;
use crate::val::{UntypedRetVal, Val};
use crate::vmctx::{Vmctx, VmctxInternal};
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

/// a representation of AsyncContext which can be freely cloned
#[doc(hidden)]
#[derive(Clone)]
pub struct AsyncContext {
    waker: Waker,
}

const DEFAULT_INST_COUNT_BOUND: u64 = i64::MAX as u64;

/// A value representing that the guest instance yielded because it was blocked on a future.
///
/// In the future, we could provide a value from the guest that can be accessed before resuming the future,
/// such as if we wanted to do something from the host context.
struct AsyncYielded;

/// Providing the private `AsyncResume` as a resume value certifies that
/// RunAsync upheld the invarriants necessary to safely resume the instance.
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
    /// function itself as async. The `#[lucet_hostcall]` macro simply calls this function.
    ///
    /// There's no performance penalty for doing so: futures that are immediately ready without waiting
    /// don't require a context switch, just like using `.await`.
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
    /// The primary reason you may want to use `try_block_on` manually is to provide a fallback
    /// implementation for if your hostcall is called from outside of an asynchronous context.
    ///
    /// If `Instance::run_async` is not being used to run the VM, this function will return
    /// `Err(BlockOnError::NeedsAsyncContext)`.
    #[doc(hidden)]
    pub fn try_block_on<R>(&self, mut f: impl Future<Output = R>) -> Result<R, BlockOnError> {
        // We pin the future to the stack (a requirement for being able to poll the future).
        // By pinning to the stack instead of using `Box::pin`, we avoid heap allocations for immediately-ready futures.
        //
        // SAFETY: We must uphold the invariants of Pin, namely that future does not move until it is dropped.
        // By overriding the variable named `f`, it is impossible to access f again, except through the pinned reference.
        let mut f = unsafe { Pin::new_unchecked(&mut f) };

        loop {
            // Get the AsyncContext, or die if we aren't async
            let cx = match &self.instance().state {
                State::Running {
                    async_context: Some(cx),
                } => cx,
                State::Running {
                    async_context: None,
                } => return Err(BlockOnError::NeedsAsyncContext),
                _ => unreachable!("Access to vmctx implies instance is Running"),
            };

            // build an std::task::Context
            let mut cx = Context::from_waker(&cx.waker);

            match f.as_mut().poll(&mut cx) {
                Poll::Ready(ret) => return Ok(ret),
                Poll::Pending => {
                    // The future is pending, so we need to yield to the async executor
                    self.yield_impl::<AsyncYielded, AsyncResume>(AsyncYielded, false, false);

                    // Providing the private `AsyncResume` as a resume value certifies that
                    // RunAsync upheld the invarriants necessary for us to avoid a borrow check.
                    //
                    // If we resume with any other value, the instance may have been modified, and it is
                    // unsound to resume the instance.
                    let AsyncResume = self.take_resumed_val::<AsyncResume>();
                }
            }
        }
    }
}

impl InstanceHandle {
    /// Run a WebAssembly function with arguments in the guest context at the given entrypoint.
    ///
    /// This method is similar to `Instance::run()`, but allows the Wasm program to invoke async hostcalls
    /// and provides the trampoline that `.await`s those futures on behalf of the guest.
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
    /// See `[RunAsync]` for details.
    ///
    /// If `runtime_bound` is provided, it will also pause the Wasm execution and yield a future
    /// that resumes it after (approximately) that many Wasm opcodes have executed.
    ///
    /// # `Vmctx` Restrictions
    ///
    /// This method permits the use of async hostcalls, but disallows all other uses of `Vmctx::
    /// yield_val_expecting_val` and family (`Vmctx::yield_`, `Vmctx::yield_expecting_val`,
    /// `Vmctx::yield_val`).
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

/// A future implementation that enables running a guest instance which can call async hostcalls.
///
/// Lucet hostcalls are synchronous `extern "C" fn` functions called from WebAssembly. In that
/// context, we cannot use `.await` directly because the hostcall is not `async`. While we could
/// block on an executor such as `futures::executor::block_on`, that would block the OS thread,
/// preventing us from running other async tasks while awaiting. This means an application will need more
/// threads than otherwise would be necessary.
///
/// `RunAsync` allows the guest to call async hostcalls just as if the guest had called the async function
/// and immediately `.await`ed it.
///
/// To define a async hostcall, simply add the async modifier to a hostcall definition:
///
/// ```ignore
/// #[lucet_hostcall]
/// #[no_mangle]
/// pub async fn hostcall_async(vmctx: &Vmctx) {
///    foobar().await
/// }
/// ```
///
/// Note: Async hostcalls may only be used if `Instance::run_async` was used to run the VM,
/// otherwise it will terminate the instance with `TerminationDetails::BlockOnNeedsAsync`.
///
/// Behind the scenes, lucet polls the future from within the guest execution context. If the future is not immediately ready,
/// the instance will yield and return control to the async executor. Later, when the future is ready to make progress,
/// the async executor will return to the guest context, where lucet will poll the future to completion.
///
/// Just like `.await`, there is no overhead for futures that are immediately ready (such as `async { 5 }`).
///
/// For async hostcalls that may yield to the async executor many times, it's recommended that you use `tokio::spawn`,
/// or the equivalent from your async executor, which will spawn the task to be run from the host execution context.
/// This avoids the overhead of context switching into the guest execution context every time the future needs to make progress.
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
    /// The instance is currently blocked on a future.
    ///
    /// We keep the async yielded around - although the value of AsyncYielded isn't used for
    /// anything, there's not much additional overhead in keeping it around, and it gives us
    /// the option to pass data from the yield and potentially add other types of execution
    /// in the future (such as bringing back host-context future execution).
    BlockedOnFuture(Box<AsyncYielded>),
    BoundExpired,
    Failed(Error),
}

impl<'a> Future for RunAsync<'a> {
    type Output = Result<UntypedRetVal, Error>;

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
            RunAsyncState::BlockedOnFuture(_) => {
                // Resume the instance and poll the future
                self.inst
                    .resume_with_val_impl(AsyncResume, Some(cx), Some(inst_count_bound))
            }
            RunAsyncState::BoundExpired => self.inst.resume_bounded(cx, inst_count_bound),
            RunAsyncState::Failed(err) => Err(err),
        };

        let res = match run_result {
            Ok(InternalRunResult::Normal(RunResult::Returned(rval))) => Ok(rval),
            Ok(InternalRunResult::Normal(RunResult::Yielded(yval))) => {
                match yval.downcast::<AsyncYielded>() {
                    Ok(ye) => {
                        self.state = RunAsyncState::BlockedOnFuture(ye);
                        return Poll::Pending;
                    }
                    Err(_) => {
                        // Any other yielded value is not supported - die with an error.
                        Err(Error::Unsupported(
                            "cannot yield anything besides a future in Instance::run_async"
                                .to_owned(),
                        ))
                    }
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
