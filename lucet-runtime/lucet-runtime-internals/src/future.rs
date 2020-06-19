use crate::error::Error;
use crate::instance::Instance;
use crate::instance::RunResult;
use crate::val::Val;
use crate::vmctx::Vmctx;
use futures::future::{FutureExt, LocalBoxFuture};
use std::any::Any;
use std::future::Future;

struct YieldedFuture(LocalBoxFuture<'static, ResumeVal>);
struct ResumeVal(Box<dyn Any + 'static>);

impl Vmctx {
    pub fn run_await<'a, R>(&'a self, f: impl Future<Output = R> + 'a) -> R
    where
        R: Any + 'static,
    {
        let f = async move { ResumeVal(Box::new(f.await)) }.boxed_local();
        let f = unsafe {
            std::mem::transmute::<LocalBoxFuture<'a, ResumeVal>, LocalBoxFuture<'static, ResumeVal>>(
                f,
            )
        };
        let ResumeVal(v) = self.yield_val_expecting_val(YieldedFuture(f));
        *v.downcast().expect("return type of run_await is incorrect")
    }
}

impl Instance {
    pub async fn run_async<'a>(
        &mut self,
        entrypoint: &str,
        args: &[Val],
    ) -> Result<RunResult, Error> {
        if self.is_yielded() {
            return Err(Error::Unsupported(
                "cannot run_async a yielded instance".to_owned(),
            ));
        }

        let mut resume_val: Option<ResumeVal> = None;
        loop {
            let run = if self.is_yielded() {
                self.resume_with_val(
                    resume_val
                        .take()
                        .expect("is_yielded implies resume_value is some"),
                )
            } else {
                self.run(entrypoint, args)
            };
            match run {
                Ok(run_result) => {
                    if run_result.has_yielded::<YieldedFuture>() {
                        let YieldedFuture(future) = *run_result
                            .yielded()
                            .unwrap()
                            .downcast::<YieldedFuture>()
                            .unwrap();
                        let future = future as LocalBoxFuture<'a, ResumeVal>;
                        resume_val = Some(future.await);
                        continue;
                    } else {
                        return Ok(run_result);
                    }
                }
                _ => return run,
            }
        }
    }
}
