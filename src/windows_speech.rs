use std::{
    sync::atomic::{AtomicU64, Ordering},
    sync::{OnceLock, mpsc},
    thread,
    time::{Duration, Instant},
};

use windows::Media::{
    Core::MediaSource,
    Playback::MediaPlayer,
    SpeechSynthesis::SpeechSynthesizer,
};
use windows::Win32::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
use windows_core::{Error, HRESULT, HSTRING, Result};

struct SpeechEngine {
    synthesizer: SpeechSynthesizer,
    player: MediaPlayer,
}

enum SpeechCommand {
    Speak { sequence: u64, text: String },
    Flush(mpsc::SyncSender<()>),
}

struct SpeechDispatcher {
    sender: mpsc::Sender<SpeechCommand>,
}

static DISPATCHER: OnceLock<SpeechDispatcher> = OnceLock::new();
static SPEECH_REQUEST_COUNT: AtomicU64 = AtomicU64::new(0);
static SPEECH_OUTPUT_COUNT: AtomicU64 = AtomicU64::new(0);
static SPEECH_FAILURE_COUNT: AtomicU64 = AtomicU64::new(0);
static SPEECH_QUEUE_FAILURE_COUNT: AtomicU64 = AtomicU64::new(0);

fn create_engine() -> Result<SpeechEngine> {
    let synthesizer = SpeechSynthesizer::new()?;
    let player = MediaPlayer::new()?;
    Ok(SpeechEngine {
        synthesizer,
        player,
    })
}

fn render(engine: &SpeechEngine, sequence: u64, text: &str) {
    let result = (|| -> Result<()> {
        let request = HSTRING::from(text);
        let operation = engine.synthesizer.SynthesizeTextToStreamAsync(&request)?;
        let deadline = Instant::now() + Duration::from_secs(5);

        let stream = loop {
            match operation.GetResults() {
                Ok(stream) => break stream,
                Err(error) if Instant::now() < deadline => {
                    let _ = error;
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(error),
            }
        };

        let content_type = stream.ContentType()?;
        let source = MediaSource::CreateFromStream(&stream, &content_type)?;
        engine.player.SetSource(&source)?;
        engine.player.Play()?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            SPEECH_OUTPUT_COUNT.fetch_add(1, Ordering::Relaxed);
            println!(
                "SPEECH_OUTPUT #{sequence} = PASS | {}",
                crate::windows_diagnostics::text(text)
            );
        }
        Err(error) => {
            SPEECH_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
            eprintln!("SPEECH_OUTPUT #{sequence} = FAIL | {error}");
        }
    }
}

fn speech_worker(receiver: mpsc::Receiver<SpeechCommand>, ready: mpsc::SyncSender<bool>) {
    let com_initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED as u32).ok().is_ok() };
    if !com_initialized {
        let _ = ready.send(false);
        return;
    }

    let engine = match create_engine() {
        Ok(engine) => engine,
        Err(error) => {
            eprintln!("SPEECH_WORKER_INIT = FAIL | {error}");
            let _ = ready.send(false);
            unsafe { CoUninitialize() };
            return;
        }
    };

    let _ = ready.send(true);
    println!("SPEECH_WORKER = READY");

    while let Ok(command) = receiver.recv() {
        match command {
            SpeechCommand::Speak { sequence, text } => render(&engine, sequence, &text),
            SpeechCommand::Flush(done) => {
                let _ = done.send(());
            }
        }
    }

    unsafe { CoUninitialize() };
}

pub fn initialize() -> Result<()> {
    if DISPATCHER.get().is_some() {
        return Ok(());
    }

    let (sender, receiver) = mpsc::channel();
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    thread::Builder::new()
        .name("screenreader-speech".to_string())
        .spawn(move || speech_worker(receiver, ready_tx))
        .map_err(|_| Error::from_hresult(HRESULT(0x80004005_u32 as i32)))?;

    match ready_rx.recv_timeout(Duration::from_secs(10)) {
        Ok(true) => {}
        _ => return Err(Error::from_hresult(HRESULT(0x80004005_u32 as i32))),
    }

    DISPATCHER
        .set(SpeechDispatcher { sender })
        .map_err(|_| Error::from_hresult(HRESULT(0x80004005_u32 as i32)))?;

    println!("SPEECH_OUTPUT_INIT = PASS");
    println!("SPEECH_DISPATCH = ASYNC_WORKER");
    crate::windows_diagnostics::print_policy_marker();
    Ok(())
}

pub fn speak(text: &str) {
    let text = text.trim();
    if text.is_empty() {
        return;
    }

    let sequence = SPEECH_REQUEST_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    println!(
        "SPEECH_REQUEST #{sequence} | {}",
        crate::windows_diagnostics::text(text)
    );

    let Some(dispatcher) = DISPATCHER.get() else {
        SPEECH_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
        eprintln!("SPEECH_OUTPUT #{sequence} = FAIL | dispatcher-not-initialized");
        return;
    };

    if dispatcher
        .sender
        .send(SpeechCommand::Speak {
            sequence,
            text: text.to_string(),
        })
        .is_err()
    {
        SPEECH_QUEUE_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
        SPEECH_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
        eprintln!("SPEECH_OUTPUT #{sequence} = FAIL | speech-worker-disconnected");
    }
}

fn flush() {
    let Some(dispatcher) = DISPATCHER.get() else {
        return;
    };
    let (done_tx, done_rx) = mpsc::sync_channel(1);
    if dispatcher.sender.send(SpeechCommand::Flush(done_tx)).is_err() {
        SPEECH_QUEUE_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
        return;
    }
    if done_rx.recv_timeout(Duration::from_secs(30)).is_err() {
        SPEECH_QUEUE_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
        eprintln!("SPEECH_FLUSH = FAIL | timeout");
    } else {
        println!("SPEECH_FLUSH = PASS");
    }
}

pub fn print_summary() {
    flush();
    println!(
        "SPEECH_COUNTS | requested={} | output={} | failed={}",
        SPEECH_REQUEST_COUNT.load(Ordering::Relaxed),
        SPEECH_OUTPUT_COUNT.load(Ordering::Relaxed),
        SPEECH_FAILURE_COUNT.load(Ordering::Relaxed)
    );
    println!(
        "SPEECH_QUEUE_COUNTS | enqueue_failures={}",
        SPEECH_QUEUE_FAILURE_COUNT.load(Ordering::Relaxed)
    );
}
