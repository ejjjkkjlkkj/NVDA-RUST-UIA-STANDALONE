#[cfg(windows)]
#[path = "../windows_speech.rs"]
mod windows_speech;

#[cfg(windows)]
fn main() -> windows_core::Result<()> {
    windows_speech::initialize()?;

    for index in 1..=20 {
        windows_speech::speak_latest(&format!("rapid selection {index}"));
    }

    windows_speech::print_summary();
    println!("SPEECH_REPLACE_SMOKE = COMPLETE");
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    println!("SPEECH_REPLACE_SMOKE = WINDOWS_ONLY");
}
