#[cfg(windows)]
mod windows_app;

#[cfg(windows)]
fn main() -> windows_core::Result<()> {
    windows_app::run()
}

#[cfg(not(windows))]
fn main() {
    println!("nvda-rust-uia-standalone: UI Automation runtime requires Windows.");
}
