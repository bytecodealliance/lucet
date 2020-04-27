use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_arch = "x86")] {
        pub mod i686;
        pub use i686 as arch_impl;
    } else if #[cfg(target_arch = "x86_64")] {
        pub mod x86_64;
        pub use x86_64 as arch_impl;
    }
}

pub mod val {
    use crate::archdeps::arch_impl;
    pub use arch_impl::val::Val as Val;
    pub use arch_impl::val::RegVal as RegVal;
    pub use arch_impl::val::UntypedRetVal as UntypedRetVal;
    pub(crate) use arch_impl::val::UntypedRetValInternal as UntypedRetValInternal;
    pub use arch_impl::val::val_to_reg as val_to_reg;
    pub use arch_impl::val::val_to_stack as val_to_stack;
}
