#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "freebsd")]
mod freebsd;

#[cfg(unix)]
mod unix;

#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "freebsd")]
pub use freebsd::*;

#[cfg(unix)]
pub use unix::*;
