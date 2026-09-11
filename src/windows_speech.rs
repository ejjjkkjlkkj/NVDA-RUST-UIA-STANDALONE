use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicU64, Ordering};

use windows::Media::{
    Core::MediaSource,
    Playback::MediaPlayer,
    SpeechSynthesis::SpeechSynthesizer,
};
use windows_core::{HSTRING, Result};

struct SpeechEngine {
    synthesizer: SpeechSynthesizer,
    player: MediaPlayer,
}

static ENGINE: OnceLock<Mutex<SpeechEngine>> = OnceLock::new();
static SPEECH_REQUEST_COUNT: AtomicU64 = AtomicU64::new(0);
static SPEECH_OUTPUT_COUNT: AtomicU64 = AtomicU64::new(0);
static SPEECH_FAILURE_COUNT: AtomicU64 = AtomicU64::new(0);

pub fn initialize() -> Result<()> {
    if ENGINE.get().is_some() {
        return Ok(());
    }

    let synthesizer = SpeechSynthesizer::new()?;
    let player = MediaPlayer::new()?;

    let _ = ENGINE.set(Mutex::new(SpeechEngine {
        synthesizer,
        player,
    }));

    println!("SPEECH_OUTPUT_INIT = PASS");
    Ok(())
}

pub fn speak(text: &str) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }

    let sequence = SPEECH_REQUEST_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    println!("SPEECH_REQUEST #{sequence} | {text}");

    let Some(engine) = ENGINE.get() else {
        SPEECH_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
        eprintln!("SPEECH_OUTPUT #{sequence} = FAIL | engine-not-initialized");
        return;
    };

    let Ok(engine) = engine.lock() else {
        SPEECH_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
        eprintln!("SPEECH_OUTPUT #{sequence} = FAIL | engine-lock-poisoned");
        return;
    };

    let result = (|| -> Result<()> {
        let request = HSTRING::from(text);
        let stream = engine
            .synthesizer
            .SynthesizeTextToStreamAsync(&request)?
            .get()?;
        let content_type = stream.ContentType()?;
        let source = MediaSource::CreateFromStream(&stream, &content_type)?;
        engine.player.SetSource(&source)?;
        engine.player.Play()?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            SPEECH_OUTPUT_COUNT.fetch_add(1, Ordering::Relaxed);
            println!("SPEECH_OUTPUT #{sequence} = PASS | {text}");
        }
        Err(error) => {
            SPEECH_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
            eprintln!("SPEECH_OUTPUT #{sequence} = FAIL | {error}");
        }
    }
}

pub fn print_summary() {
    println!(
        "SPEECH_COUNTS | requested={} | output={} | failed={}",
        SPEECH_REQUEST_COUNT.load(Ordering::Relaxed),
        SPEECH_OUTPUT_COUNT.load(Ordering::Relaxed),
        SPEECH_FAILURE_COUNT.load(Ordering::Relaxed)
    );
}
