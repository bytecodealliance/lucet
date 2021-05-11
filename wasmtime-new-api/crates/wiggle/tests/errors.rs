/// Execute the wiggle guest conversion code to exercise it
mod convert_just_errno {
    use wiggle_test::{impl_errno, HostMemory, WasiCtx};

    /// The `errors` argument to the wiggle gives us a hook to map a rich error
    /// type like this one (typical of wiggle use cases in wasi-common and beyond)
    /// down to the flat error enums that witx can specify.
    #[derive(Debug, thiserror::Error)]
    pub enum RichError {
        #[error("Invalid argument: {0}")]
        InvalidArg(String),
        #[error("Won't cross picket line: {0}")]
        PicketLine(String),
    }

    // Define an errno with variants corresponding to RichError. Use it in a
    // trivial function.
    wiggle::from_witx!({
        witx_literal: "
(typename $errno (enum (@witx tag u8) $ok $invalid_arg $picket_line))
(module $one_error_conversion
  (@interface func (export \"foo\")
     (param $strike u32)
     (result $err (expected (error $errno)))))
    ",
        errors: { errno => RichError },
    });

    impl_errno!(types::Errno);

    /// When the `errors` mapping in witx is non-empty, we need to impl the
    /// types::UserErrorConversion trait that wiggle generates from that mapping.
    impl<'a> types::UserErrorConversion for WasiCtx<'a> {
        fn errno_from_rich_error(&mut self, e: RichError) -> Result<types::Errno, wiggle::Trap> {
            // WasiCtx can collect a Vec<String> log so we can test this. We're
            // logging the Display impl that `thiserror::Error` provides us.
            self.log.borrow_mut().push(e.to_string());
            // Then do the trivial mapping down to the flat enum.
            match e {
                RichError::InvalidArg { .. } => Ok(types::Errno::InvalidArg),
                RichError::PicketLine { .. } => Ok(types::Errno::PicketLine),
            }
        }
    }

    impl<'a> one_error_conversion::OneErrorConversion for WasiCtx<'a> {
        fn foo(&mut self, strike: u32) -> Result<(), RichError> {
            // We use the argument to this function to exercise all of the
            // possible error cases we could hit here
            match strike {
                0 => Ok(()),
                1 => Err(RichError::PicketLine(format!("I'm not a scab"))),
                _ => Err(RichError::InvalidArg(format!("out-of-bounds: {}", strike))),
            }
        }
    }

    #[test]
    fn one_error_conversion_test() {
        let mut ctx = WasiCtx::new();
        let host_memory = HostMemory::new();

        // Exercise each of the branches in `foo`.
        // Start with the success case:
        let r0 = one_error_conversion::foo(&mut ctx, &host_memory, 0);
        assert_eq!(
            r0,
            Ok(types::Errno::Ok as i32),
            "Expected return value for strike=0"
        );
        assert!(ctx.log.borrow().is_empty(), "No error log for strike=0");

        // First error case:
        let r1 = one_error_conversion::foo(&mut ctx, &host_memory, 1);
        assert_eq!(
            r1,
            Ok(types::Errno::PicketLine as i32),
            "Expected return value for strike=1"
        );
        assert_eq!(
            ctx.log.borrow_mut().pop().expect("one log entry"),
            "Won't cross picket line: I'm not a scab",
            "Expected log entry for strike=1",
        );

        // Second error case:
        let r2 = one_error_conversion::foo(&mut ctx, &host_memory, 2);
        assert_eq!(
            r2,
            Ok(types::Errno::InvalidArg as i32),
            "Expected return value for strike=2"
        );
        assert_eq!(
            ctx.log.borrow_mut().pop().expect("one log entry"),
            "Invalid argument: out-of-bounds: 2",
            "Expected log entry for strike=2",
        );
    }
}

/// Type-check the wiggle guest conversion code against a more complex case where
/// we use two distinct error types.
mod convert_multiple_error_types {
    pub use super::convert_just_errno::RichError;
    use wiggle_test::{impl_errno, WasiCtx};

    /// Test that we can map multiple types of errors.
    #[derive(Debug, thiserror::Error)]
    #[allow(dead_code)]
    pub enum AnotherRichError {
        #[error("I've had this many cups of coffee and can't even think straight: {0}")]
        TooMuchCoffee(usize),
    }

    // Just like the prior test, except that we have a second errno type. This should mean there
    // are two functions in UserErrorConversion.
    // Additionally, test that the function "baz" marked noreturn always returns a wiggle::Trap.
    wiggle::from_witx!({
        witx_literal: "
(typename $errno (enum (@witx tag u8) $ok $invalid_arg $picket_line))
(typename $errno2 (enum (@witx tag u8) $ok $too_much_coffee))
(module $two_error_conversions
  (@interface func (export \"foo\")
     (param $strike u32)
     (result $err (expected (error $errno))))
  (@interface func (export \"bar\")
     (param $drink u32)
     (result $err (expected (error $errno2))))
  (@interface func (export \"baz\")
     (param $drink u32)
     (@witx noreturn)))
    ",
        errors: { errno => RichError, errno2 => AnotherRichError },
    });

    impl_errno!(types::Errno);
    impl_errno!(types::Errno2);

    // The UserErrorConversion trait will also have two methods for this test. They correspond to
    // each member of the `errors` mapping.
    // Bodies elided.
    impl<'a> types::UserErrorConversion for WasiCtx<'a> {
        fn errno_from_rich_error(&mut self, _e: RichError) -> Result<types::Errno, wiggle::Trap> {
            unimplemented!()
        }
        fn errno2_from_another_rich_error(
            &mut self,
            _e: AnotherRichError,
        ) -> Result<types::Errno2, wiggle::Trap> {
            unimplemented!()
        }
    }

    // And here's the witx module trait impl, bodies elided
    impl<'a> two_error_conversions::TwoErrorConversions for WasiCtx<'a> {
        fn foo(&mut self, _: u32) -> Result<(), RichError> {
            unimplemented!()
        }
        fn bar(&mut self, _: u32) -> Result<(), AnotherRichError> {
            unimplemented!()
        }
        fn baz(&mut self, _: u32) -> wiggle::Trap {
            unimplemented!()
        }
    }
}
