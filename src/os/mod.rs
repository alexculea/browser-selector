pub mod shared;

#[cfg(target_os = "windows")]
mod win;
#[cfg(target_os = "windows")]
pub use win::util::*;
#[cfg(target_os = "windows")]
pub use win::sys_browsers;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::sys_browsers;

#[cfg(target_os = "macos")]
pub use macos::util::*;