#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(unix)]
mod unix;

#[cfg(unix)]
mod mmap;

#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(unix)]
pub use unix::*;

#[cfg(unix)]
pub use mmap::*;
