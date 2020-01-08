/// The macro that surrounds definitions of Lucet hostcalls in Rust.
///
/// **Note:** this macro has been deprecated and replaced by the `#[lucet_hostcall]` attribute.
///
/// It is important to use this macro for hostcalls, rather than exporting them directly, as it
/// installs unwind protection that prevents panics from unwinding into the guest stack.
///
/// Since this is not a proc macro, the syntax is unfortunately fairly brittle. The functions it
/// encloses must be of the form:
///
/// ```ignore
/// #[$attr1]
/// #[$attr2]
/// ... // any number of attributes are supported; in most cases you will want `#[no_mangle]`
/// pub unsafe extern "C" fn $name( // must be `pub unsafe extern "C"`
///     &mut $vmctx,
///     $arg1: $arg1_ty,
///     $arg2: $arg2_ty,
///     ... , // trailing comma must always be present
/// ) -> $ret_ty { // return type must always be present even if it is `()`
///     // body
/// }
/// ```
#[macro_export]
#[deprecated(since = "0.5.0", note = "Use the #[lucet_hostcall] attribute instead")]
macro_rules! lucet_hostcalls {
    {
        $(
            $(#[$attr:meta])*
            pub unsafe extern "C" fn $name:ident(
                &mut $vmctx:ident
                $(, $arg:ident : $arg_ty:ty )*,
            ) -> $ret_ty:ty {
                $($body:tt)*
            }
        )*
    } => {
        $(
            #[allow(unused_mut)]
            #[allow(unused_unsafe)]
            #[$crate::lucet_hostcall]
            $(#[$attr])*
            pub unsafe extern "C" fn $name(
                $vmctx: &mut lucet_runtime::vmctx::Vmctx,
                $( $arg: $arg_ty ),*
            ) -> $ret_ty {
                $($body)*
            }
        )*
    }
}

/// Terminate an instance from within a hostcall, returning an optional value as an error.
///
/// Use this instead of `panic!` when you want the instance to terminate, but not the entire host
/// program. Like `panic!`, you can pass a format string with arguments, a value that implements
/// `Any`, or nothing to return a default message.
///
/// Upon termination, the call to `Instance::run()` will return with an
/// `Err(Error::RuntimeTerminated)` value containing the value you pass to this macro.
///
/// This macro safely unwinds the hostcall stack out to the entrypoint of the hostcall, so any
/// resources that may have been acquired will be properly dropped.
#[macro_export]
macro_rules! lucet_hostcall_terminate {
    () => {
        lucet_hostcall_terminate!("lucet_hostcall_terminate")
    };
    ( $payload:expr ) => {
        panic!($crate::instance::TerminationDetails::provide($payload))
    };
    ( $payload:expr, ) => {
        lucet_hostcall_terminate!($payload)
    };
    ( $fmt:expr, $($arg:tt)+ ) => {
        lucet_hostcall_terminate!(format!($fmt, $($arg),+))
    };
}
