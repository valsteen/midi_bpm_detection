#[cfg(target_os = "macos")]
use core_foundation_sys::base::OSStatus;

#[cfg(target_os = "macos")]
fn main() -> Result<(), OSStatus> {
    coremidi::restart()
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("This utility only works on macOS");
    std::process::exit(1);
}
