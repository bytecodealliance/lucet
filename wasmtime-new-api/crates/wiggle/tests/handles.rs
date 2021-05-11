use proptest::prelude::*;
use wiggle::{GuestMemory, GuestType};
use wiggle_test::{impl_errno, HostMemory, MemArea, WasiCtx};

const FD_VAL: u32 = 123;

wiggle::from_witx!({
    witx: ["$CARGO_MANIFEST_DIR/tests/handles.witx"],
});

impl_errno!(types::Errno);

impl<'a> handle_examples::HandleExamples for WasiCtx<'a> {
    fn fd_create(&mut self) -> Result<types::Fd, types::Errno> {
        Ok(types::Fd::from(FD_VAL))
    }
    fn fd_consume(&mut self, fd: types::Fd) -> Result<(), types::Errno> {
        println!("FD_CONSUME {}", fd);
        if fd == types::Fd::from(FD_VAL) {
            Ok(())
        } else {
            Err(types::Errno::InvalidArg)
        }
    }
}

#[derive(Debug)]
struct HandleExercise {
    pub return_loc: MemArea,
}

impl HandleExercise {
    pub fn test(&self) {
        let mut ctx = WasiCtx::new();
        let host_memory = HostMemory::new();

        let e = handle_examples::fd_create(&mut ctx, &host_memory, self.return_loc.ptr as i32);

        assert_eq!(e, Ok(types::Errno::Ok as i32), "fd_create error");

        let h_got: u32 = host_memory
            .ptr(self.return_loc.ptr)
            .read()
            .expect("return ref_mut");

        assert_eq!(h_got, 123, "fd_create return val");

        let e = handle_examples::fd_consume(&mut ctx, &host_memory, h_got as i32);

        assert_eq!(e, Ok(types::Errno::Ok as i32), "fd_consume error");

        let e = handle_examples::fd_consume(&mut ctx, &host_memory, h_got as i32 + 1);

        assert_eq!(
            e,
            Ok(types::Errno::InvalidArg as i32),
            "fd_consume invalid error"
        );
    }

    pub fn strat() -> BoxedStrategy<Self> {
        (HostMemory::mem_area_strat(types::Fd::guest_size()))
            .prop_map(|return_loc| HandleExercise { return_loc })
            .boxed()
    }
}

proptest! {
    #[test]
    fn handle_exercise(e in HandleExercise::strat()) {
        e.test()
    }
}
