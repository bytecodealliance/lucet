use crate::error::Error;
use crate::instance::{InstanceHandle, InternalRunResult, RunResult, State, TerminationDetails};
use crate::module::FunctionHandle;
use crate::val::{UntypedRetVal, Val};
use crate::vmctx::{Vmctx, VmctxInternal};
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::{
    cell::UnsafeCell,
    mem::{transmute, MaybeUninit},
};

const DEFAULT_INST_COUNT_BOUND: u64 = i64::MAX as u64;

/// A unique type that wraps a future and its returned value
///
/// Type and lifetime guarantees are maintained by `Vmctx::block_on` and `Instance::run_async`. The
/// user never sees this type.
struct YieldedFuture<'a>(Pin<&'a mut (dyn Future<Output = ResumeAsync>)>);

unsafe impl<'a> Send for YieldedFuture<'a> {}

/// The return value for a blocked async.
///
/// SAFETY: should only be constructed if the future has been polled to completion
struct ResumeAsync;

impl Vmctx {
    /// Block on the result of an `async` computation from an instance run by `Instance::run_async`.
    ///
    /// Lucet hostcalls are synchronous `extern "C" fn` functions called from WebAssembly. In that
    /// context, we cannot use `.await` directly because the hostcall is not `async`. While we could
    /// block on an executor using `futures::executor::block_on` or
    /// `tokio::runtime::Runtime::block_on`, that has two drawbacks:
    ///
    /// - If the Lucet instance was originally invoked from an async context, trying to block on the
    ///   same runtime will fail if the executor cannot be nested (all executors we know of have this
    ///   restriction).
    ///
    /// - The current OS thread would be blocked on the result of the computation, rather than being
    ///   able to run other async tasks while awaiting. This means an application will need more
    ///   threads than otherwise would be necessary.
    ///
    /// Instead, this block_on operator is designed to work only when called within an invocation
    /// of [`Instance::run_async`]. The `run_async` method executes instance code within a
    /// trampoline, itself running within an async context, making it possible to temporarily pause
    /// guest execution, jump back to the trampoline, and await there. The future given to block_on
    /// is in passed back to that trampoline, and runs on the same runtime that invoked
    /// `run_async`, avoiding problems of nesting, and allowing the current OS thread to continue
    /// performing other async work.
    ///
    /// Note:
    /// - This method may only be used if `Instance::run_async` was used to run the VM,
    ///   otherwise it will terminate the instance with `TerminationDetails::BlockOnNeedsAsync`.
    /// - This method is not reentrant. Use `.await` rather than `block_on` within the future.
    ///   Calling block_on from within the future will result in a panic.
    /// - It is not valid to re-enter guest code from the future, as guest execution may be paused.
    pub fn block_on<R>(&self, f: impl Future<Output = R> + Send) -> R
    where
        R: Any + Send + 'static,
    {
        // Get the std::task::Context, or die if we aren't async
        let mut cx = match &self.instance().state {
            State::Running {
                async_context: Some(cx),
            } => cx.borrow_mut(),
            State::Running {
                async_context: None,
            } => {
                panic!(TerminationDetails::BlockOnNeedsAsync);
            }
            _ => unreachable!("Access to vmctx implies instance is Running"),
        };

        // Wrap the future, so that we don't have to worry about sending back the return value
        let ret = UnsafeCell::new(MaybeUninit::uninit());

        let ret_ptr = ret.get();

        let mut f = async move {
            let ret = f.await;
            // SAFETY: we are the only possible writer to ret_ptr, and the future must be polled to completion before this function returns
            unsafe {
                std::ptr::write(ret_ptr, MaybeUninit::new(ret));
            }
            ResumeAsync
        };

        // We pin the future to the stack (a requirement for being able to poll the future).
        // By pinning to the stack instead of using `Box::pin`, we avoid heap allocations for immediately-ready futures.
        //
        // SAFETY: We must uphold the invariants of Pin, namely that future does not move until it is dropped.
        // By overriding the variable named `f`, it is impossible to access f again, except through the pinned reference.
        let mut f = unsafe { Pin::new_unchecked(&mut f) };

        if let Poll::Pending = f.as_mut().poll(*cx) {
            // The future is pending, so we need to yield it to the async executor to be polled to completion

            // SAFTEY: YieldedFuture is marked Send, which would not normally be the case due to ownership of ret_ptr, a raw pointer.
            // Safe because:
            // 1) We ensure that the inner future is Send
            // 2) The pointer is only written to once, preventing race conditions
            // 3) the pointer is not read from until after the future is polled to completion
            let f = YieldedFuture(f);

            // We need to lie about this lifetime so that `YieldedFuture` may be passed through the yield.
            // `Instance::run_async` rehydrates this lifetime to be at most as long as the Vmctx's `'_`.
            // This is safe because the stack frame that `'_` is tied to gets frozen in place as part of `self.yield_val_expecting_val`.
            let f = unsafe { transmute::<YieldedFuture<'_>, YieldedFuture<'static>>(f) };

            // Yield so that `Instance::run_async` can catch and execute our future.
            self.yield_impl::<YieldedFuture<'static>, ResumeAsync>(f, false, false);

            // Resuming with a ResumeAsync asserts that the future has been polled to completion
            let ResumeAsync = self.take_resumed_val();
        }

        // SAFETY: the future must have been polled to completion
        unsafe { ret.into_inner().assume_init() }
    }
}

impl InstanceHandle {
    /// Run a WebAssembly function with arguments in the guest context at the given entrypoint.
    ///
    /// This method is similar to `Instance::run()`, but allows the Wasm program to invoke hostcalls
    /// that use `Vmctx::block_on` and provides the trampoline that `.await`s those futures on
    /// behalf of the guest.
    ///
    /// If `runtime_bound` is provided, it will also pause the Wasm execution and yield a future
    /// that resumes it after (approximately) that many Wasm opcodes have executed.
    ///
    /// # `Vmctx` Restrictions
    ///
    /// This method permits the use of `Vmctx::block_on`, but disallows all other uses of `Vmctx::
    /// yield_val_expecting_val` and family (`Vmctx::yield_`, `Vmctx::yield_expecting_val`,
    /// `Vmctx::yield_val`).
    pub fn run_async<'a>(&'a mut self, entrypoint: &'a str, args: &'a [Val]) -> RunAsync<'a> {
        let func = self.module.get_export_func(entrypoint);
        self.run_async_internal(func, args)
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
    pub fn run_async_start<'a>(&'a mut self) -> RunAsync<'a> {
        let func = if self.is_not_started() {
            self.module
                .get_start_func()
                // Invariant: can only be in NotStarted state if a start function exists
                .map(|start| start.expect("NotStarted, but no start function"))
        } else {
            Err(Error::StartAlreadyRun)
        };

        self.run_async_internal(func, &[])
    }

    fn run_async_internal<'a>(
        &'a mut self,
        func: Result<FunctionHandle, Error>,
        args: &'a [Val],
    ) -> RunAsync<'a> {
        let state = match func {
            Ok(func) => RunAsyncState::Start(func, args),
            Err(err) => RunAsyncState::Failed(Some(err)),
        };

        RunAsync {
            inst: self,
            inst_count_bound: DEFAULT_INST_COUNT_BOUND,
            state,
        }
    }
}

pub struct RunAsync<'a> {
    inst: &'a mut InstanceHandle,
    state: RunAsyncState<'a>,
    /// The instance count bound. Can be changed at any time, taking effect on the next guest entry
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
    Blocked(YieldedFuture<'a>),
    Yielded,
    Failed(Option<Error>),
}

impl<'a> Future for RunAsync<'a> {
    type Output = Result<UntypedRetVal, Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inst_count_bound = self.inst_count_bound;

        let run_result = match self.state {
            RunAsyncState::Failed(ref mut err) => {
                return Poll::Ready(Err(err.take().expect("failed future polled twice")))
            }
            RunAsyncState::Start(func, args) => {
                // This is the first iteration, call the entrypoint:
                self.inst
                    .run_func(func, args, Some(cx), Some(inst_count_bound))
            }
            RunAsyncState::Blocked(ref mut fut) => {
                let resume = match fut.0.as_mut().poll(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(resume) => resume,
                };

                // Resume the instance now that the future is ready
                self.inst
                    .resume_with_val_impl(resume, Some(cx), Some(inst_count_bound))
            }
            RunAsyncState::Yielded => self.inst.resume_bounded(inst_count_bound),
        };

        match run_result {
            Ok(InternalRunResult::Normal(RunResult::Returned(rval))) => {
                // Finished running, return UntypedReturnValue
                return Poll::Ready(Ok(rval));
            }
            Ok(InternalRunResult::Normal(RunResult::Yielded(yval))) => {
                match yval.downcast::<YieldedFuture<'static>>() {
                    Ok(future) => {
                        // Rehydrate the lifetime from `'static` to `'a`, which
                        // is morally the same lifetime as was passed into
                        // `Vmctx::block_on`.
                        let future = unsafe {
                            transmute::<YieldedFuture<'static>, YieldedFuture<'a>>(*future)
                        };

                        self.state = RunAsyncState::Blocked(future);
                    }
                    Err(_) => {
                        // Any other yielded value is not supported - die with an error.
                        return Poll::Ready(Err(Error::Unsupported(
                            "cannot yield anything besides a future in Instance::run_async"
                                .to_owned(),
                        )));
                    }
                }
            }
            Ok(InternalRunResult::BoundExpired) => {
                self.state = RunAsyncState::Yielded;

                // Yield, giving control back to the async executor
                cx.waker().wake_by_ref();
            }
            Err(err) => return Poll::Ready(Err(err)),
        }

        return Poll::Pending;
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
