use std::{
    env,
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::Duration,
};

use nvda_rust_uia_standalone::{
    AccessibilityEventKind, ElementSnapshot, format_event_line, parse_monitor_seconds,
};
use windows::Win32::*;
use windows_core::{BSTR, Interface, Ref, Result, implement};

const TEXT_SELECTION_CHANGED_EVENT: EVENTID = EVENTID(20014);
const TEXT_CHANGED_EVENT: EVENTID = EVENTID(20015);

const PROCESS_ID_PROPERTY: PROPERTYID = PROPERTYID(30002);
const LOCALIZED_CONTROL_TYPE_PROPERTY: PROPERTYID = PROPERTYID(30004);
const NAME_PROPERTY: PROPERTYID = PROPERTYID(30005);
const AUTOMATION_ID_PROPERTY: PROPERTYID = PROPERTYID(30011);
const CLASS_NAME_PROPERTY: PROPERTYID = PROPERTYID(30012);
const FRAMEWORK_ID_PROPERTY: PROPERTYID = PROPERTYID(30024);
const VALUE_VALUE_PROPERTY: PROPERTYID = PROPERTYID(30045);
const SELECTION_ITEM_IS_SELECTED_PROPERTY: PROPERTYID = PROPERTYID(30079);
const TOGGLE_TOGGLE_STATE_PROPERTY: PROPERTYID = PROPERTYID(30086);
const PROPERTY_CHANGE_PROPERTIES: [PROPERTYID; 3] = [
    VALUE_VALUE_PROPERTY,
    SELECTION_ITEM_IS_SELECTED_PROPERTY,
    TOGGLE_TOGGLE_STATE_PROPERTY,
];

static FOCUS_COUNT: AtomicU64 = AtomicU64::new(0);
static TEXT_CHANGED_COUNT: AtomicU64 = AtomicU64::new(0);
static TEXT_SELECTION_COUNT: AtomicU64 = AtomicU64::new(0);
static PROPERTY_CHANGED_COUNT: AtomicU64 = AtomicU64::new(0);
static CACHE_FULL_HIT_EVENTS: AtomicU64 = AtomicU64::new(0);
static CACHE_FALLBACK_PROPERTIES: AtomicU64 = AtomicU64::new(0);

struct ComApartment;

impl ComApartment {
    unsafe fn initialize_mta() -> Result<Self> {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED as u32).ok()? };
        Ok(Self)
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

fn bstr_or_unavailable(value: Result<BSTR>) -> String {
    value
        .map(|value| value.display().to_string())
        .unwrap_or_else(|_| "<unavailable>".to_string())
}

unsafe fn create_event_cache(automation: &IUIAutomation) -> Result<IUIAutomationCacheRequest> {
    let cache = unsafe { automation.CreateCacheRequest()? };

    for property in [
        PROCESS_ID_PROPERTY,
        FRAMEWORK_ID_PROPERTY,
        CLASS_NAME_PROPERTY,
        LOCALIZED_CONTROL_TYPE_PROPERTY,
        NAME_PROPERTY,
        AUTOMATION_ID_PROPERTY,
    ] {
        unsafe { cache.AddProperty(property).ok()? };
    }

    Ok(cache)
}

fn event_cache(automation: &IUIAutomation) -> Option<IUIAutomationCacheRequest> {
    match unsafe { create_event_cache(automation) } {
        Ok(cache) => {
            println!("UIA_EVENT_PROPERTY_CACHE = ENABLED");
            Some(cache)
        }
        Err(error) => {
            eprintln!("UIA_EVENT_PROPERTY_CACHE = FALLBACK_CURRENT | {error}");
            None
        }
    }
}

fn snapshot_sender(sender: Ref<IUIAutomationElement>) -> Option<ElementSnapshot> {
    let Some(element) = sender.as_ref() else {
        return None;
    };

    unsafe {
        let mut fallback_properties = 0_u64;

        let process_id = match element.CachedProcessId() {
            Ok(value) => value,
            Err(_) => {
                fallback_properties += 1;
                element.CurrentProcessId().unwrap_or_default()
            }
        };

        let framework = match element.CachedFrameworkId() {
            Ok(value) => value.display().to_string(),
            Err(_) => {
                fallback_properties += 1;
                bstr_or_unavailable(element.CurrentFrameworkId())
            }
        };

        let class_name = match element.CachedClassName() {
            Ok(value) => value.display().to_string(),
            Err(_) => {
                fallback_properties += 1;
                bstr_or_unavailable(element.CurrentClassName())
            }
        };

        let role = match element.CachedLocalizedControlType() {
            Ok(value) => value.display().to_string(),
            Err(_) => {
                fallback_properties += 1;
                bstr_or_unavailable(element.CurrentLocalizedControlType())
            }
        };

        let name = match element.CachedName() {
            Ok(value) => value.display().to_string(),
            Err(_) => {
                fallback_properties += 1;
                bstr_or_unavailable(element.CurrentName())
            }
        };

        let automation_id = match element.CachedAutomationId() {
            Ok(value) => value.display().to_string(),
            Err(_) => {
                fallback_properties += 1;
                bstr_or_unavailable(element.CurrentAutomationId())
            }
        };

        if fallback_properties == 0 {
            CACHE_FULL_HIT_EVENTS.fetch_add(1, Ordering::Relaxed);
        } else {
            CACHE_FALLBACK_PROPERTIES.fetch_add(fallback_properties, Ordering::Relaxed);
        }

        Some(ElementSnapshot {
            process_id,
            framework,
            class_name,
            role,
            name,
            automation_id,
        })
    }
}

fn print_sender(kind: AccessibilityEventKind, sequence: u64, sender: Ref<IUIAutomationElement>) {
    let Some(snapshot) = snapshot_sender(sender) else {
        eprintln!("{kind} #{sequence} | sender=NULL");
        return;
    };

    println!("{}", format_event_line(kind, sequence, &snapshot));
}

fn print_property_sender(
    sequence: u64,
    property_id: PROPERTYID,
    sender: Ref<IUIAutomationElement>,
) {
    let Some(snapshot) = snapshot_sender(sender) else {
        eprintln!(
            "PROPERTY_CHANGED/UIA_{} #{sequence} | sender=NULL",
            property_id.0
        );
        return;
    };

    let property_name = match property_id {
        VALUE_VALUE_PROPERTY => "ValueValue",
        SELECTION_ITEM_IS_SELECTED_PROPERTY => "SelectionItemIsSelected",
        TOGGLE_TOGGLE_STATE_PROPERTY => "ToggleToggleState",
        _ => "Unknown",
    };

    println!(
        "PROPERTY_CHANGED/UIA_{}[{property_name}] #{sequence} | PID={} | Framework={} | Class={} | Role={} | Name={} | AutomationId={}",
        property_id.0,
        snapshot.process_id,
        snapshot.framework,
        snapshot.class_name,
        snapshot.role,
        snapshot.name,
        snapshot.automation_id
    );
}

#[implement(IUIAutomationFocusChangedEventHandler)]
struct FocusSink;

impl IUIAutomationFocusChangedEventHandler_Impl for FocusSink_Impl {
    fn HandleFocusChangedEvent(&self, sender: Ref<IUIAutomationElement>) -> Result<()> {
        let sequence = FOCUS_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        print_sender(AccessibilityEventKind::Focus, sequence, sender);
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
            print_sender(AccessibilityEventKind::TextChanged, sequence, sender);
        } else if eventid == TEXT_SELECTION_CHANGED_EVENT {
            let sequence = TEXT_SELECTION_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
            print_sender(
                AccessibilityEventKind::TextSelectionChanged,
                sequence,
                sender,
            );
        }

        Ok(())
    }
}

#[implement(IUIAutomationPropertyChangedEventHandler)]
struct PropertyChangedSink;

impl IUIAutomationPropertyChangedEventHandler_Impl for PropertyChangedSink_Impl {
    fn HandlePropertyChangedEvent(
        &self,
        sender: Ref<IUIAutomationElement>,
        propertyid: PROPERTYID,
        _newvalue: &VARIANT,
    ) -> Result<()> {
        let sequence = PROPERTY_CHANGED_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        print_property_sender(sequence, propertyid, sender);
        Ok(())
    }
}

unsafe fn run_modern(
    seconds: u64,
    focus_handler: &IUIAutomationFocusChangedEventHandler,
    automation_handler: &IUIAutomationEventHandler,
    property_handler: &IUIAutomationPropertyChangedEventHandler,
) -> Result<()> {
    let automation6: IUIAutomation6 =
        unsafe { CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)? };

    let automation: IUIAutomation = automation6.cast()?;
    let root = unsafe { automation.GetRootElement()? };
    let group = unsafe { automation6.CreateEventHandlerGroup()? };
    let cache = event_cache(&automation);

    unsafe {
        group
            .AddAutomationEventHandler(
                TEXT_CHANGED_EVENT,
                TreeScope_Subtree,
                cache.as_ref(),
                automation_handler,
            )
            .ok()?;

        group
            .AddAutomationEventHandler(
                TEXT_SELECTION_CHANGED_EVENT,
                TreeScope_Subtree,
                cache.as_ref(),
                automation_handler,
            )
            .ok()?;

        automation6.AddEventHandlerGroup(&root, &group).ok()?;

        automation
            .AddPropertyChangedEventHandlerNativeArray(
                &root,
                TreeScope_Subtree,
                cache.as_ref(),
                property_handler,
                &PROPERTY_CHANGE_PROPERTIES,
            )
            .ok()?;
    }

    if let Err(error) = unsafe {
        automation
            .AddFocusChangedEventHandler(cache.as_ref(), focus_handler)
            .ok()
    } {
        let _ = unsafe {
            automation
                .RemovePropertyChangedEventHandler(&root, property_handler)
                .ok()
        };
        let _ = unsafe { automation6.RemoveEventHandlerGroup(&root, &group).ok() };
        return Err(error);
    }

    println!("UIA_ACTIVATION = CUIAutomation8");
    println!("UIA_INTERFACE = IUIAutomation6");
    println!("EVENT_REGISTRATION = HANDLER_GROUP_PLUS_PROPERTY_CHANGED");
    println!("UIA_NATIVE_EVENTS_INIT = PASS");
    println!("MONITOR_SECONDS = {seconds}");
    println!("TEST = focus + text input + caret/selection + value/state changes");

    thread::sleep(Duration::from_secs(seconds));

    unsafe {
        automation
            .RemoveFocusChangedEventHandler(focus_handler)
            .ok()?;
        automation
            .RemovePropertyChangedEventHandler(&root, property_handler)
            .ok()?;
        automation6.RemoveEventHandlerGroup(&root, &group).ok()?;
    }

    thread::sleep(Duration::from_millis(300));
    Ok(())
}

unsafe fn create_compat_automation() -> Result<IUIAutomation> {
    match unsafe {
        CoCreateInstance::<_, IUIAutomation>(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
    } {
        Ok(automation) => {
            println!("UIA_COMPAT_ACTIVATION = CUIAutomation8/IUIAutomation");
            Ok(automation)
        }
        Err(first_error) => {
            eprintln!(
                "CUIAutomation8/IUIAutomation unavailable: {first_error}; fallback CUIAutomation"
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
    property_handler: &IUIAutomationPropertyChangedEventHandler,
) -> Result<()> {
    let automation = unsafe { create_compat_automation()? };
    let root = unsafe { automation.GetRootElement()? };
    let cache = event_cache(&automation);

    unsafe {
        automation
            .AddAutomationEventHandler(
                TEXT_CHANGED_EVENT,
                &root,
                TreeScope_Subtree,
                cache.as_ref(),
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
                cache.as_ref(),
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
            .AddPropertyChangedEventHandlerNativeArray(
                &root,
                TreeScope_Subtree,
                cache.as_ref(),
                property_handler,
                &PROPERTY_CHANGE_PROPERTIES,
            )
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

    if let Err(error) = unsafe {
        automation
            .AddFocusChangedEventHandler(cache.as_ref(), focus_handler)
            .ok()
    } {
        let _ = unsafe {
            automation
                .RemovePropertyChangedEventHandler(&root, property_handler)
                .ok()
        };
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
    println!("EVENT_REGISTRATION = INDIVIDUAL_COMPAT_PLUS_PROPERTY_CHANGED");
    println!("UIA_NATIVE_EVENTS_INIT = PASS");
    println!("MONITOR_SECONDS = {seconds}");
    println!("TEST = focus + text input + caret/selection + value/state changes");

    thread::sleep(Duration::from_secs(seconds));

    unsafe {
        automation
            .RemoveFocusChangedEventHandler(focus_handler)
            .ok()?;
        automation
            .RemovePropertyChangedEventHandler(&root, property_handler)
            .ok()?;
        automation
            .RemoveAutomationEventHandler(TEXT_SELECTION_CHANGED_EVENT, &root, automation_handler)
            .ok()?;
        automation
            .RemoveAutomationEventHandler(TEXT_CHANGED_EVENT, &root, automation_handler)
            .ok()?;
    }

    thread::sleep(Duration::from_millis(300));
    Ok(())
}

pub fn run() -> Result<()> {
    let monitor_arg = env::args().nth(1);
    let seconds = parse_monitor_seconds(monitor_arg.as_deref());

    unsafe {
        let _com = ComApartment::initialize_mta()?;

        let focus_handler: IUIAutomationFocusChangedEventHandler = FocusSink.into();
        let automation_handler: IUIAutomationEventHandler = AutomationSink.into();
        let property_handler: IUIAutomationPropertyChangedEventHandler = PropertyChangedSink.into();

        match run_modern(
            seconds,
            &focus_handler,
            &automation_handler,
            &property_handler,
        ) {
            Ok(()) => {
                println!("RUNTIME_MODE = MODERN_IUIAUTOMATION6");
            }
            Err(error) => {
                eprintln!("MODERN_UIA6_UNAVAILABLE = {error}");
                eprintln!("FALLBACK = IUIAutomation individual event registration");

                run_compat(
                    seconds,
                    &focus_handler,
                    &automation_handler,
                    &property_handler,
                )?;
                println!("RUNTIME_MODE = COMPAT_IUIAUTOMATION");
            }
        }

        println!(
            "COUNTS | focus={} | text_changed={} | text_selection={} | property_changed={}",
            FOCUS_COUNT.load(Ordering::Relaxed),
            TEXT_CHANGED_COUNT.load(Ordering::Relaxed),
            TEXT_SELECTION_COUNT.load(Ordering::Relaxed),
            PROPERTY_CHANGED_COUNT.load(Ordering::Relaxed)
        );
        println!(
            "UIA_EVENT_CACHE_COUNTS | full_hit_events={} | fallback_properties={}",
            CACHE_FULL_HIT_EVENTS.load(Ordering::Relaxed),
            CACHE_FALLBACK_PROPERTIES.load(Ordering::Relaxed)
        );
        println!("UIA_NATIVE_EVENTS_RUNTIME = PASS");
    }

    Ok(())
}
