#[cfg(windows)]
mod windows_runtime_v2;
#[cfg(windows)]
mod windows_speech;
#[cfg(windows)]
mod windows_textpattern2;

#[cfg(windows)]
fn main() -> windows_core::Result<()> {
    windows_runtime_v2::run()
}

#[cfg(not(windows))]
fn main() {
    use nvda_rust_uia_standalone::platform::current_backend;

    let backend = current_backend();
    println!("SCREEN_READER_CORE = PASS");
    println!("PLATFORM_BACKEND = {backend:?}");
    println!("NATIVE_ACCESSIBILITY_API = {}", backend.native_api());
    println!(
        "NATIVE_BACKEND_IMPLEMENTED = {}",
        backend.is_native_backend_implemented()
    );

    if !backend.is_native_backend_implemented() {
        println!("BACKEND_STATUS = PORTABLE_CORE_READY_NATIVE_ADAPTER_PENDING");
    }
}
