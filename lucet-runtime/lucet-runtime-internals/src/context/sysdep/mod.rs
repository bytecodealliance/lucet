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
