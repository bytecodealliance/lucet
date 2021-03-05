use crate::error::Error;
use crate::instance::{InstanceHandle, InternalRunResult, RunResult, State, TerminationDetails};
use crate::module::FunctionHandle;
use crate::val::{UntypedRetVal, Val};
use crate::vmctx::{Vmctx, VmctxInternal};
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// This is the same type defined by the `futures` library, but we don't need the rest of the
/// library for this purpose.
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

/// A unique type that wraps a boxed future with a boxed return value.
///
/// Type and lifetime guarantees are maintained by `Vmctx::block_on` and `Instance::run_async`. The
/// user never sees this type.
struct YieldedFuture(BoxFuture<'static, ResumeVal>);

/// A unique type for a boxed return value. The user never sees this type.
pub struct ResumeVal(Box<dyn Any + 'static>);

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
    /// Note that this method may only be used if `Instance::run_async` was used to run the VM,
    /// otherwise it will terminate the instance with `TerminationDetails::BlockOnNeedsAsync`.
    pub fn block_on<'a, R>(&'a self, f: impl Future<Output = R> + 'a) -> R
    where
        R: Any + 'static,
    {
        // Die if we aren't in Instance::run_async
        match self.instance().state {
            State::Running { async_context } => {
                if !async_context {
                    panic!(TerminationDetails::BlockOnNeedsAsync)
                }
            }
            _ => unreachable!("Access to vmctx implies instance is Running"),
        }
        // Wrap the Output of `f` as a boxed ResumeVal. Then, box the entire
        // async computation.
        let f = Box::pin(async move { ResumeVal(Box::new(f.await)) });
        // Change the lifetime of the async computation from `'a` to `'static.
        // We need to lie about this lifetime so that `YieldedFuture` may impl
        // `Any` and be passed through the yield. `Instance::run_async`
        // rehydrates this lifetime to be at most as long as the Vmctx's `'a`.
        // This is safe because the stack frame that `'a` is tied to gets
        // frozen in place as part of `self.yield_val_expecting_val`.
        let f = unsafe {
            std::mem::transmute::<BoxFuture<'a, ResumeVal>, BoxFuture<'static, ResumeVal>>(f)
        };
        // Wrap the computation in `YieldedFuture` so that
        // `Instance::run_async` can catch and run it.  We will get the
        // `ResumeVal` we applied to `f` above.
        self.yield_impl::<YieldedFuture, ResumeVal>(YieldedFuture(f), false, false);
        let ResumeVal(v) = self.take_resumed_val();
        // We may now downcast and unbox the returned Box<dyn Any> into an `R`
        // again.
        *v.downcast().expect("run_async broke invariant")
    }
}

/// A simple future that yields once. We use this to yield when a runtime bound is reached.
///
/// Inspired by Tokio's `yield_now()`.
struct YieldNow {
    yielded: bool,
}

impl YieldNow {
    fn new() -> Self {
        Self { yielded: false }
    }
}

impl Future for YieldNow {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.yielded = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
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
    pub async fn run_async<'a>(
        &'a mut self,
        entrypoint: &'a str,
        args: &'a [Val],
        runtime_bound: Option<u64>,
    ) -> Result<UntypedRetVal, Error> {
        let func = self.module.get_export_func(entrypoint)?;
        self.run_async_internal(func, args, runtime_bound).await
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
    pub async fn run_async_start<'a>(
        &'a mut self,
        runtime_bound: Option<u64>,
    ) -> Result<(), Error> {
        if !self.is_not_started() {
            return Err(Error::StartAlreadyRun);
        }
        let start = match self.module.get_start_func()? {
            Some(start) => start,
            None => return Ok(()),
        };
        self.run_async_internal(start, &[], runtime_bound).await?;
        Ok(())
    }

    /// Shared async run-loop implementation for both `run_async()` and
    /// `run_start_async()`.
    async fn run_async_internal<'a>(
        &'a mut self,
        func: FunctionHandle,
        args: &'a [Val],
        runtime_bound: Option<u64>,
    ) -> Result<UntypedRetVal, Error> {
        if self.is_yielded() {
            return Err(Error::Unsupported(
                "cannot run_async a yielded instance".to_owned(),
            ));
        }

        // Store the ResumeVal here when we get it.
        let mut resume_val: Option<ResumeVal> = None;
        loop {
            // Run the WebAssembly program
            let run_result = if self.is_yielded() {
                // A previous iteration of the loop stored the ResumeVal in
                // `resume_val`, send it back to the guest ctx and continue
                // running:
                self.resume_with_val_impl(
                    resume_val
                        .take()
                        .expect("is_yielded implies resume_value is some"),
                    true,
                    runtime_bound,
                )
            } else if self.is_bound_expired() {
                self.resume_bounded(
                    runtime_bound.expect("should have bound if guest had expired bound"),
                )
            } else {
                // This is the first iteration, call the entrypoint:
                self.run_func(func, args, true, runtime_bound)
            };
            match run_result? {
                InternalRunResult::Normal(RunResult::Returned(rval)) => {
                    // Finished running, return UntypedReturnValue
                    return Ok(rval);
                }
                InternalRunResult::Normal(RunResult::Yielded(yval)) => {
                    // Check if the yield came from Vmctx::block_on:
                    if yval.is::<YieldedFuture>() {
                        let YieldedFuture(future) = *yval.downcast::<YieldedFuture>().unwrap();
                        // Rehydrate the lifetime from `'static` to `'a`, which
                        // is morally the same lifetime as was passed into
                        // `Vmctx::block_on`.
                        let future = future as BoxFuture<'a, ResumeVal>;

                        // await on the computation. Store its result in
                        // `resume_val`.
                        resume_val = Some(future.await);
                    // Now we want to `Instance::resume_with_val` and start
                    // this cycle over.
                    } else {
                        // Any other yielded value is not supported - die with an error.
                        return Err(Error::Unsupported(
                            "cannot yield anything besides a future in Instance::run_async"
                                .to_owned(),
                        ));
                    }
                }
                InternalRunResult::BoundExpired => {
                    // Await on a simple future that yields once then is ready.
                    YieldNow::new().await
                }
            }
        }
    }
}
