use crate::error::Error;
use crate::instance::{InstanceHandle, RunResult, State, TerminationDetails};
use crate::val::{UntypedRetVal, Val};
use crate::vmctx::{Vmctx, VmctxInternal};
use std::any::Any;
use std::future::Future;
use std::pin::Pin;

/// This is the same type defined by the `futures` library, but we don't need the rest of the
/// library for this purpose.
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A unique type that wraps a boxed future with a boxed return value.
///
/// Type and lifetime guarantees are maintained by `Vmctx::block_on` and `Instance::run_async`. The
/// user never sees this type.
struct YieldedFuture(BoxFuture<'static, ResumeVal>);

/// A unique type for a boxed return value. The user never sees this type.
pub struct ResumeVal(Box<dyn Any + Send + 'static>);

unsafe impl Send for ResumeVal {}

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
    pub fn block_on<'a, R>(&'a self, f: impl Future<Output = R> + Send + 'a) -> R
    where
        R: Any + Send + 'static,
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
        let ResumeVal(v) = self.yield_val_expecting_val(YieldedFuture(f));
        // We may now downcast and unbox the returned Box<dyn Any> into an `R`
        // again.
        *v.downcast().expect("run_async broke invariant")
    }
}

/// This enum is used internally to `InstanceHandle::run_async`. It is only
/// `pub` in order for the type signature of a "block_in_place" function to be
/// writable, which is a concession because Rust does not have rank 2 types.
/// The user should never inspect or construct the contents of this enum.
pub enum Bounce<'a> {
    Done(UntypedRetVal),
    More(BoxFuture<'a, ResumeVal>),
}

impl InstanceHandle {
    /// Run a WebAssembly function with arguments in the guest context at the given entrypoint.
    ///
    /// This method is similar to `Instance::run()`, but allows the Wasm program to invoke hostcalls
    /// that use `Vmctx::block_on` and provides the trampoline that `.await`s those futures on
    /// behalf of the guest.
    ///
    /// # `Vmctx` Restrictions
    ///
    /// This method permits the use of `Vmctx::block_on`, but disallows all other uses of `Vmctx::
    /// yield_val_expecting_val` and family (`Vmctx::yield_`, `Vmctx::yield_expecting_val`,
    /// `Vmctx::yield_val`).
    ///
    /// # Blocking thread
    ///
    /// The `wrap_blocking` argument is a function that is called with a closure that runs the Wasm
    /// program. Since Wasm may execute for an arbitrarily long time without `await`ing, we need to
    /// make sure that it runs on a thread that is allowed to block.
    ///
    /// This argument is designed with [`tokio::task::block_in_place`][tokio] in mind. The odd type
    /// is a concession to the fact that we don't have rank 2 types in Rust, and so must fall back
    /// to trait objects in order to be able to take an argument that is itself a function that
    /// takes a closure.
    ///
    /// In order to provide an appropriate function, you may have to wrap the library function in
    /// another closure so that the types are compatible. For example:
    ///
    /// ```no_run
    /// # async fn f() {
    /// # let instance: lucet_runtime_internals::instance::InstanceHandle = unimplemented!();
    /// fn block_in_place<F, R>(f: F) -> R
    /// where
    ///     F: FnOnce() -> R,
    /// {
    ///     // ...
    ///     # f()
    /// }
    ///
    /// instance.run_async("entrypoint", &[], |f| block_in_place(f)).await.unwrap();
    /// # }
    /// ```
    ///
    /// [tokio]: https://docs.rs/tokio/0.2.21/tokio/task/fn.block_in_place.html
    pub async fn run_async<'a, F>(
        &'a mut self,
        entrypoint: &'a str,
        args: &'a [Val],
        wrap_blocking: F,
    ) -> Result<UntypedRetVal, Error>
    where
        F: for<'b> Fn(&mut (dyn FnMut() -> Result<Bounce<'b>, Error>)) -> Result<Bounce<'b>, Error>,
    {
        if self.is_yielded() {
            return Err(Error::Unsupported(
                "cannot run_async a yielded instance".to_owned(),
            ));
        }

        // Store the ResumeVal here when we get it.
        let mut resume_val: Option<ResumeVal> = None;
        loop {
            // Run the WebAssembly program
            let bounce = wrap_blocking(&mut || {
                let run_result = if self.is_yielded() {
                    // A previous iteration of the loop stored the ResumeVal in
                    // `resume_val`, send it back to the guest ctx and continue
                    // running:
                    self.resume_with_val_impl(
                        resume_val
                            .take()
                            .expect("is_yielded implies resume_value is some"),
                        true,
                    )
                } else {
                    // This is the first iteration, call the entrypoint:
                    let func = self.module.get_export_func(entrypoint)?;
                    self.run_func(func, args, true)
                };
                match run_result? {
                    RunResult::Returned(rval) => {
                        // Finished running, return UntypedReturnValue
                        return Ok(Bounce::Done(rval));
                    }
                    RunResult::Yielded(yval) => {
                        // Check if the yield came from Vmctx::block_on:
                        if yval.is::<YieldedFuture>() {
                            let YieldedFuture(future) = *yval.downcast::<YieldedFuture>().unwrap();
                            // Rehydrate the lifetime from `'static` to `'a`, which
                            // is morally the same lifetime as was passed into
                            // `Vmctx::block_on`.
                            Ok(Bounce::More(future as BoxFuture<'a, ResumeVal>))
                        } else {
                            // Any other yielded value is not supported - die with an error.
                            Err(Error::Unsupported(
                                "cannot yield anything besides a future in Instance::run_async"
                                    .to_owned(),
                            ))
                        }
                    }
                }
            })?;
            match bounce {
                Bounce::Done(rval) => return Ok(rval),
                Bounce::More(fut) => {
                    // await on the computation. Store its result in
                    // `resume_val`.
                    resume_val = Some(fut.await);
                    // Now we want to `Instance::resume_with_val` and start
                    // this cycle over.
                }
            }
        }
    }
}
