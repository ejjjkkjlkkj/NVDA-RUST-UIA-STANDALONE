use std::{
    env,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::Duration,
};

use windows::Win32::*;
use windows_core::{BSTR, Interface, Ref, Result, implement};

const TEXT_SELECTION_CHANGED_EVENT: EVENTID = EVENTID(20014);
const TEXT_CHANGED_EVENT: EVENTID = EVENTID(20015);

static FOCUS_COUNT: AtomicU64 = AtomicU64::new(0);
static TEXT_CHANGED_COUNT: AtomicU64 = AtomicU64::new(0);
static TEXT_SELECTION_COUNT: AtomicU64 = AtomicU64::new(0);

fn bstr_or_unavailable(value: Result<BSTR>) -> String {
    value
        .map(|value| value.display().to_string())
        .unwrap_or_else(|_| "<unavailable>".to_string())
}

fn print_sender(kind: &str, sequence: u64, sender: Ref<IUIAutomationElement>) {
    let Some(element) = sender.as_ref() else {
        eprintln!("{kind} #{sequence} | sender=NULL");
        return;
    };

    unsafe {
        let name = bstr_or_unavailable(element.CurrentName());
        let class_name = bstr_or_unavailable(element.CurrentClassName());
        let framework = bstr_or_unavailable(element.CurrentFrameworkId());
        let automation_id = bstr_or_unavailable(element.CurrentAutomationId());
        let control_type = bstr_or_unavailable(element.CurrentLocalizedControlType());
        let process_id = element.CurrentProcessId().unwrap_or_default();

        println!(
            "{kind} #{sequence} | PID={process_id} | Framework={framework} | Class={class_name} | Role={control_type} | Name={name} | AutomationId={automation_id}"
        );
    }
}

#[implement(IUIAutomationFocusChangedEventHandler)]
struct FocusSink;

impl IUIAutomationFocusChangedEventHandler_Impl for FocusSink_Impl {
    fn HandleFocusChangedEvent(&self, sender: Ref<IUIAutomationElement>) -> Result<()> {
        let sequence = FOCUS_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        print_sender("FOCUS", sequence, sender);
        Ok(())
    }
}

#[implement(IUIAutomationEventHandler)]
struct AutomationSink;

impl IUIAutomationEventHandler_Impl for AutomationSink_Impl {
    fn HandleAutomationEvent(
        &self,
        sender: Ref<IUIAutomationElement>,
        eventid: EVENTID,
    ) -> Result<()> {
        if eventid == TEXT_CHANGED_EVENT {
            let sequence = TEXT_CHANGED_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
            print_sender("TEXT_CHANGED/UIA_20015", sequence, sender);
        } else if eventid == TEXT_SELECTION_CHANGED_EVENT {
            let sequence = TEXT_SELECTION_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
            print_sender("TEXT_SELECTION_CHANGED/UIA_20014", sequence, sender);
        }

        Ok(())
    }
}

unsafe fn run_modern(
    seconds: u64,
    focus_handler: &IUIAutomationFocusChangedEventHandler,
    automation_handler: &IUIAutomationEventHandler,
) -> Result<()> {
    // IMPORTANT:
    // IUIAutomation6 est demande directement au coclass moderne CUIAutomation8.
    let automation6: IUIAutomation6 =
        unsafe { CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)? };

    let automation: IUIAutomation = automation6.cast()?;
    let root = unsafe { automation.GetRootElement()? };

    let group = unsafe { automation6.CreateEventHandlerGroup()? };

    unsafe {
        group
            .AddAutomationEventHandler(
                TEXT_CHANGED_EVENT,
                TreeScope_Subtree,
                None::<&IUIAutomationCacheRequest>,
                automation_handler,
            )
            .ok()?;

        group
            .AddAutomationEventHandler(
                TEXT_SELECTION_CHANGED_EVENT,
                TreeScope_Subtree,
                None::<&IUIAutomationCacheRequest>,
                automation_handler,
            )
            .ok()?;

        automation6.AddEventHandlerGroup(&root, &group).ok()?;
    }

    if let Err(error) = unsafe {
        automation
            .AddFocusChangedEventHandler(None::<&IUIAutomationCacheRequest>, focus_handler)
            .ok()
    } {
        let _ = unsafe { automation6.RemoveEventHandlerGroup(&root, &group).ok() };
        return Err(error);
    }

    println!("UIA_ACTIVATION = CUIAutomation8");
    println!("UIA_INTERFACE = IUIAutomation6");
    println!("EVENT_REGISTRATION = HANDLER_GROUP");
    println!("UIA_NATIVE_EVENTS_INIT = PASS");
    println!("MONITOR_SECONDS = {seconds}");
    println!("TEST = focus + frappe texte + curseur/selection");

    thread::sleep(Duration::from_secs(seconds));

    unsafe {
        automation6.RemoveEventHandlerGroup(&root, &group).ok()?;
        automation
            .RemoveFocusChangedEventHandler(focus_handler)
            .ok()?;
    }

    thread::sleep(Duration::from_millis(300));

    Ok(())
}

unsafe fn create_compat_automation() -> Result<IUIAutomation> {
    // D'abord le coclass moderne, mais sans exiger IUIAutomation6.
    match unsafe {
        CoCreateInstance::<_, IUIAutomation>(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
    } {
        Ok(automation) => {
            println!("UIA_COMPAT_ACTIVATION = CUIAutomation8/IUIAutomation");
            Ok(automation)
        }
        Err(first_error) => {
            eprintln!(
                "CUIAutomation8/IUIAutomation indisponible: {first_error}; fallback CUIAutomation"
            );

            unsafe {
                CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
            }
        }
    }
}

unsafe fn run_compat(
    seconds: u64,
    focus_handler: &IUIAutomationFocusChangedEventHandler,
    automation_handler: &IUIAutomationEventHandler,
) -> Result<()> {
    let automation = unsafe { create_compat_automation()? };
    let root = unsafe { automation.GetRootElement()? };

    unsafe {
        automation
            .AddAutomationEventHandler(
                TEXT_CHANGED_EVENT,
                &root,
                TreeScope_Subtree,
                None::<&IUIAutomationCacheRequest>,
                automation_handler,
            )
            .ok()?;
    }

    if let Err(error) = unsafe {
        automation
            .AddAutomationEventHandler(
                TEXT_SELECTION_CHANGED_EVENT,
                &root,
                TreeScope_Subtree,
                None::<&IUIAutomationCacheRequest>,
                automation_handler,
            )
            .ok()
    } {
        let _ = unsafe {
            automation
                .RemoveAutomationEventHandler(TEXT_CHANGED_EVENT, &root, automation_handler)
                .ok()
        };
        return Err(error);
    }

    if let Err(error) = unsafe {
        automation
            .AddFocusChangedEventHandler(None::<&IUIAutomationCacheRequest>, focus_handler)
            .ok()
    } {
        let _ = unsafe {
            automation
                .RemoveAutomationEventHandler(
                    TEXT_SELECTION_CHANGED_EVENT,
                    &root,
                    automation_handler,
                )
                .ok()
        };
        let _ = unsafe {
            automation
                .RemoveAutomationEventHandler(TEXT_CHANGED_EVENT, &root, automation_handler)
                .ok()
        };
        return Err(error);
    }

    println!("UIA_INTERFACE = IUIAutomation");
    println!("EVENT_REGISTRATION = INDIVIDUAL_COMPAT");
    println!("UIA_NATIVE_EVENTS_INIT = PASS");
    println!("MONITOR_SECONDS = {seconds}");
    println!("TEST = focus + frappe texte + curseur/selection");

    thread::sleep(Duration::from_secs(seconds));

    unsafe {
        automation
            .RemoveAutomationEventHandler(TEXT_SELECTION_CHANGED_EVENT, &root, automation_handler)
            .ok()?;

        automation
            .RemoveAutomationEventHandler(TEXT_CHANGED_EVENT, &root, automation_handler)
            .ok()?;

        automation
            .RemoveFocusChangedEventHandler(focus_handler)
            .ok()?;
    }

    thread::sleep(Duration::from_millis(300));

    Ok(())
}

fn main() -> Result<()> {
    let seconds = env::args()
        .nth(1)
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(15);

    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED as u32).ok()?;

        let focus_handler: IUIAutomationFocusChangedEventHandler = FocusSink.into();
        let automation_handler: IUIAutomationEventHandler = AutomationSink.into();

        match run_modern(seconds, &focus_handler, &automation_handler) {
            Ok(()) => {
                println!("RUNTIME_MODE = MODERN_IUIAUTOMATION6");
            }
            Err(error) => {
                eprintln!("MODERN_UIA6_UNAVAILABLE = {error}");
                eprintln!("FALLBACK = IUIAutomation individual event registration");

                run_compat(seconds, &focus_handler, &automation_handler)?;

                println!("RUNTIME_MODE = COMPAT_IUIAUTOMATION");
            }
        }

        println!(
            "COUNTS | focus={} | text_changed={} | text_selection={}",
            FOCUS_COUNT.load(Ordering::Relaxed),
            TEXT_CHANGED_COUNT.load(Ordering::Relaxed),
            TEXT_SELECTION_COUNT.load(Ordering::Relaxed)
        );

        println!("UIA_NATIVE_EVENTS_RUNTIME = PASS");

        CoUninitialize();
    }

    Ok(())
}
