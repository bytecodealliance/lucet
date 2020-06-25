use crate::error::Error;
use crate::instance::{Instance, RunResult, State, TerminationDetails};
use crate::val::Val;
use crate::vmctx::{Vmctx, VmctxInternal};
use std::any::Any;
use std::future::Future;
use std::pin::Pin;

/// This is the same type defined by the `futures` library, but we don't need the rest of the
/// library for this purpose.
type LocalBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

impl Vmctx {
    /// Run an `async` computation. A `Vmctx` is passed to ordinary
    /// (synchronous) functions called from WebAssembly. We cannot execute
    /// `async` code in that context, so this method trampolines an `async`
    /// computation back to `Instance::run_async`, which can `.await` on it.
    /// Note that this method may only be used if `Instance::run_async` was
    /// used to run the VM, otherwise it will terminate the instance.
    pub fn run_await<'a, R>(&'a self, f: impl Future<Output = R> + 'a) -> R
    where
        R: Any + 'static,
    {
        // Die if we aren't in Instance::run_async
        match self.instance().state {
            State::Running { async_context } => {
                if !async_context {
                    panic!(TerminationDetails::AwaitNeedsAsync)
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
        let f = unsafe {
            std::mem::transmute::<LocalBoxFuture<'a, ResumeVal>, LocalBoxFuture<'static, ResumeVal>>(
                f,
            )
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

impl Instance {
    /// Run a WebAssembly function with arguments in the guest context at the
    /// given entrypoint. Enable `Vmctx::run_await` to trampoline async
    /// computations back to this context so that we may `.await` on them.
    ///
    /// Aside from asynchrony, this function behaves identically to
    /// `Instance::run`.
    pub async fn run_async<'a>(
        &'a mut self,
        entrypoint: &str,
        args: &[Val],
    ) -> Result<RunResult, Error> {
        if self.is_yielded() {
            return Err(Error::Unsupported(
                "cannot run_async a yielded instance".to_owned(),
            ));
        }

        // Store the ResumeVal here when we get it.
        let mut resume_val: Option<ResumeVal> = None;
        let ret = loop {
            // Run the WebAssembly program
            let run = if self.is_yielded() {
                // A previous iteration of the loop stored the ResumeVal in
                // `resume_val`, send it back to the guest ctx and continue
                // running:
                self._resume_with_val(
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
            match run {
                Ok(run_result) => {
                    // Check if the yield came from Vmctx::run_await:
                    if run_result.has_yielded::<YieldedFuture>() {
                        let YieldedFuture(future) = *run_result
                            .yielded()
                            .unwrap()
                            .downcast::<YieldedFuture>()
                            .unwrap();
                        // Rehydrate the lifetime from `'static` to `'a`, which
                        // is morally the same lifetime as was passed into
                        // `Vmctx::run_await`.
                        let future = future as LocalBoxFuture<'a, ResumeVal>;
                        // await on the computation. Store its result in
                        // `resume_val`.
                        resume_val = Some(future.await);
                        // Now we want to `Instance::resume_with_val` and start
                        // this cycle over.
                        continue;
                    } else {
                        // Any other result of the run is returned to this
                        // function's caller.
                        break Ok(run_result);
                    }
                }
                _ => {
                    // Any other result of the run is returned to this
                    // function's caller.
                    break run;
                }
            }
        };

        // Return the result of the run.
        return ret;
    }
}

/// A unique type that wraps a boxed future with a boxed return value.
/// Type and lifetime guarantees are maintained by `Vmctx::run_await` and
/// `Instance::run_async`. The user never sees this type.
struct YieldedFuture(LocalBoxFuture<'static, ResumeVal>);
/// A unique type for a boxed return value. The user never sees this type.
struct ResumeVal(Box<dyn Any + 'static>);
