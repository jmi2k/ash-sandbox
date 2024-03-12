#[cfg(windows)]
mod win32;

#[cfg(windows)]
pub use win32::*;

#[cfg(unix)]
mod x11;

#[cfg(unix)]
pub use x11::*;
