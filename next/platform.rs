#[cfg(windows)]
#[path = "platform/windows.rs"]
mod sys;

/*
#[cfg(target_os = "linux")]
#[path = "platform/linux.rs"]
mod sys;

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly",
))]
#[path = "platform/bsd.rs"]
mod sys;
*/

pub fn main() {
    sys::main();
}
