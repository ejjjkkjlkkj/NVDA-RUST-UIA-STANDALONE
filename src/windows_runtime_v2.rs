use std::{env, thread, time::{Duration, Instant}};
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

use nvda_rust_uia_standalone::{
    AccessibilityEventKind, ElementSnapshot, format_event_line, parse_monitor_seconds,
};
use windows::Win32::*;
use windows_core::{BSTR, Ref, Result, implement};

const TEXT_SELECTION_CHANGED_EVENT: EVENTID = EVENTID(20014);
const TEXT_CHANGED_EVENT: EVENTID = EVENTID(20015);
const VALUE_PATTERN: PATTERNID = PATTERNID(10002);
const TOGGLE_PATTERN: PATTERNID = PATTERNID(10015);
const COMBO_BOX_CONTROL_TYPE_ID: i32 = 50003;
const PROCESS_ID_PROPERTY: PROPERTYID = PROPERTYID(30002);
const LOCALIZED_CONTROL_TYPE_PROPERTY: PROPERTYID = PROPERTYID(30004);
const NAME_PROPERTY: PROPERTYID = PROPERTYID(30005);
const AUTOMATION_ID_PROPERTY: PROPERTYID = PROPERTYID(30011);
const CLASS_NAME_PROPERTY: PROPERTYID = PROPERTYID(30012);
const IS_PASSWORD_PROPERTY: PROPERTYID = PROPERTYID(30019);
const FRAMEWORK_ID_PROPERTY: PROPERTYID = PROPERTYID(30024);
const VALUE_VALUE_PROPERTY: PROPERTYID = PROPERTYID(30045);
const SELECTION_ITEM_IS_SELECTED_PROPERTY: PROPERTYID = PROPERTYID(30079);
const TOGGLE_TOGGLE_STATE_PROPERTY: PROPERTYID = PROPERTYID(30086);
const PROPERTY_CHANGE_PROPERTIES: [PROPERTYID; 3] = [
    VALUE_VALUE_PROPERTY,
    SELECTION_ITEM_IS_SELECTED_PROPERTY,
    TOGGLE_TOGGLE_STATE_PROPERTY,
];
const MAX_TRACKED_CONTROL_STATES: usize = 256;

static FOCUS_COUNT: AtomicU64 = AtomicU64::new(0);
static FOCUS_EVENT_COUNT: AtomicU64 = AtomicU64::new(0);
static FOCUS_POLL_COUNT: AtomicU64 = AtomicU64::new(0);
static TEXT_CHANGED_COUNT: AtomicU64 = AtomicU64::new(0);
static TEXT_SELECTION_COUNT: AtomicU64 = AtomicU64::new(0);
static PROPERTY_CHANGED_COUNT: AtomicU64 = AtomicU64::new(0);
static PROPERTY_POLL_COUNT: AtomicU64 = AtomicU64::new(0);
static PROPERTY_EVENT_SAMPLE_COUNT: AtomicU64 = AtomicU64::new(0);
static CACHE_FULL_HIT_EVENTS: AtomicU64 = AtomicU64::new(0);
static CACHE_FALLBACK_PROPERTIES: AtomicU64 = AtomicU64::new(0);
static LAST_FOCUS_IDENTITY: Mutex<Option<String>> = Mutex::new(None);
static POLLED_STATES: Mutex<Vec<PolledState>> = Mutex::new(Vec::new());

struct ComApartment;
impl ComApartment {
    unsafe fn initialize() -> Result<Self> {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED as u32).ok()? };
        Ok(Self)
    }
}
impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

struct Observation {
    snapshot: ElementSnapshot,
    is_password: bool,
}

struct PolledState {
    identity: String,
    value: Option<String>,
    toggle: Option<ToggleState>,
}

fn bstr(value: Result<BSTR>) -> String {
    value
        .map(|value| value.display().to_string())
        .unwrap_or_else(|_| "<unavailable>".to_string())
}

fn cache(automation: &IUIAutomation) -> Option<IUIAutomationCacheRequest> {
    let result = unsafe {
        let cache = automation.CreateCacheRequest().ok()?;
        for property in [
            PROCESS_ID_PROPERTY,
            FRAMEWORK_ID_PROPERTY,
            CLASS_NAME_PROPERTY,
            LOCALIZED_CONTROL_TYPE_PROPERTY,
            NAME_PROPERTY,
            AUTOMATION_ID_PROPERTY,
            IS_PASSWORD_PROPERTY,
        ] {
            cache.AddProperty(property).ok().ok()?;
        }
        Some(cache)
    };

    if result.is_some() {
        println!("UIA_EVENT_PROPERTY_CACHE = ENABLED");
    } else {
        eprintln!("UIA_EVENT_PROPERTY_CACHE = FALLBACK_CURRENT");
    }
    result
}

fn observe(element: &IUIAutomationElement) -> Observation {
    unsafe {
        let mut fallback = 0_u64;
        macro_rules! cached_or_current {
            ($cached:expr, $current:expr, $convert:expr) => {
                match $cached {
                    Ok(value) => $convert(value),
                    Err(_) => {
                        fallback += 1;
                        $convert($current.unwrap_or_default())
                    }
                }
            };
        }

        let process_id = cached_or_current!(
            element.CachedProcessId(),
            element.CurrentProcessId(),
            |value| value
        );
        let framework = match element.CachedFrameworkId() {
            Ok(value) => value.display().to_string(),
            Err(_) => {
                fallback += 1;
                bstr(element.CurrentFrameworkId())
            }
        };
        let class_name = match element.CachedClassName() {
            Ok(value) => value.display().to_string(),
            Err(_) => {
                fallback += 1;
                bstr(element.CurrentClassName())
            }
        };
        let role = match element.CachedLocalizedControlType() {
            Ok(value) => value.display().to_string(),
            Err(_) => {
                fallback += 1;
                bstr(element.CurrentLocalizedControlType())
            }
        };
        let name = match element.CachedName() {
            Ok(value) => value.display().to_string(),
            Err(_) => {
                fallback += 1;
                bstr(element.CurrentName())
            }
        };
        let automation_id = match element.CachedAutomationId() {
            Ok(value) => value.display().to_string(),
            Err(_) => {
                fallback += 1;
                bstr(element.CurrentAutomationId())
            }
        };
        let is_password: bool = match element.CachedIsPassword() {
            Ok(value) => value.into(),
            Err(_) => {
                fallback += 1;
                element
                    .CurrentIsPassword()
                    .map(Into::into)
                    .unwrap_or(false)
            }
        };

        if fallback == 0 {
            CACHE_FULL_HIT_EVENTS.fetch_add(1, Ordering::Relaxed);
        } else {
            CACHE_FALLBACK_PROPERTIES.fetch_add(fallback, Ordering::Relaxed);
        }

        Observation {
            snapshot: ElementSnapshot {
                process_id,
                framework,
                class_name,
                role,
                name,
                automation_id,
            },
            is_password,
        }
    }
}

fn speak_focus(observation: &Observation) {
    let phrase = if observation.is_password {
        "password field".to_string()
    } else {
        let name = observation.snapshot.name.trim();
        let role = observation.snapshot.role.trim();
        if !name.is_empty() && name != "<unavailable>" && !role.is_empty() && role != "<unavailable>" {
            format!("{name}, {role}")
        } else if !name.is_empty() && name != "<unavailable>" {
            name.to_string()
        } else if !role.is_empty() && role != "<unavailable>" {
            role.to_string()
        } else {
            String::new()
        }
    };

    if !phrase.is_empty() {
        crate::windows_speech::speak(&phrase);
    }
}

fn focus_identity(observation: &Observation) -> String {
    let s = &observation.snapshot;
    format!(
        "{}|{}|{}|{}|{}|{}",
        s.process_id, s.framework, s.class_name, s.role, s.name, s.automation_id
    )
}

fn focus_changed(observation: &Observation) -> bool {
    let identity = focus_identity(observation);
    match LAST_FOCUS_IDENTITY.lock() {
        Ok(mut previous) => {
            if previous.as_deref() == Some(identity.as_str()) {
                false
            } else {
                *previous = Some(identity);
                true
            }
        }
        Err(_) => true,
    }
}

fn emit_focus(observation: &Observation, source: &str) {
    if !focus_changed(observation) {
        return;
    }

    let sequence = FOCUS_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if source == "event" {
        FOCUS_EVENT_COUNT.fetch_add(1, Ordering::Relaxed);
    } else {
        FOCUS_POLL_COUNT.fetch_add(1, Ordering::Relaxed);
    }

    let s = &observation.snapshot;
    println!(
        "FOCUS #{sequence} | Source={source} | PID={} | Framework={} | Class={} | Role={} | Name={} | AutomationId={}",
        s.process_id, s.framework, s.class_name, s.role, s.name, s.automation_id
    );
    speak_focus(observation);
}

fn property_label(property: PROPERTYID) -> &'static str {
    match property {
        VALUE_VALUE_PROPERTY => "ValueValue",
        SELECTION_ITEM_IS_SELECTED_PROPERTY => "SelectionItemIsSelected",
        TOGGLE_TOGGLE_STATE_PROPERTY => "ToggleToggleState",
        _ => "Unknown",
    }
}

fn emit_property_observation(
    sequence: u64,
    property: PROPERTYID,
    observation: &Observation,
    source: &str,
) {
    let s = &observation.snapshot;
    let label = property_label(property);
    println!(
        "PROPERTY_CHANGED/UIA_{}[{label}] #{sequence} | Source={source} | PID={} | Framework={} | Class={} | Role={} | Name={} | AutomationId={}",
        property.0, s.process_id, s.framework, s.class_name, s.role, s.name, s.automation_id
    );
}

fn sample_control_state(
    element: &IUIAutomationElement,
    observation: &Observation,
    source: &'static str,
) {
    let identity = focus_identity(observation);
    let value = if observation.is_password {
        None
    } else {
        unsafe {
            element
                .GetCurrentPatternAs::<IUIAutomationValuePattern>(VALUE_PATTERN)
                .ok()
                .and_then(|pattern| pattern.CurrentValue().ok())
                .map(|value| value.display().to_string())
        }
    };
    let toggle = unsafe {
        element
            .GetCurrentPatternAs::<IUIAutomationTogglePattern>(TOGGLE_PATTERN)
            .ok()
            .and_then(|pattern| pattern.CurrentToggleState().ok())
    };
    let is_combo_box = unsafe {
        element
            .CurrentControlType()
            .map(|control_type| control_type.0 == COMBO_BOX_CONTROL_TYPE_ID)
            .unwrap_or(false)
    };

    let Ok(mut states) = POLLED_STATES.lock() else {
        return;
    };

    let mut changed_value = None;
    let mut toggle_changed = None;
    if let Some(index) = states.iter().position(|state| state.identity == identity) {
        let old = &mut states[index];
        if old.value != value && old.value.is_some() && value.is_some() {
            changed_value = value.clone();
        }
        if old.toggle != toggle && old.toggle.is_some() && toggle.is_some() {
            toggle_changed = toggle;
        }
        old.value = value;
        old.toggle = toggle;
    } else {
        if states.len() >= MAX_TRACKED_CONTROL_STATES {
            states.remove(0);
        }
        states.push(PolledState {
            identity,
            value,
            toggle,
        });
    }
    drop(states);

    if let Some(value) = changed_value {
        let sequence = PROPERTY_CHANGED_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        if source == "poll" {
            PROPERTY_POLL_COUNT.fetch_add(1, Ordering::Relaxed);
        } else {
            PROPERTY_EVENT_SAMPLE_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        emit_property_observation(sequence, VALUE_VALUE_PROPERTY, observation, source);

        if is_combo_box {
            let phrase = value.trim();
            if !phrase.is_empty() {
                println!("VALUE_SPEECH #{sequence} | control=combo-box | value={phrase}");
                crate::windows_speech::speak(phrase);
            }
        }
    }

    if let Some(state) = toggle_changed {
        let sequence = PROPERTY_CHANGED_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        if source == "poll" {
            PROPERTY_POLL_COUNT.fetch_add(1, Ordering::Relaxed);
        } else {
            PROPERTY_EVENT_SAMPLE_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        emit_property_observation(sequence, TOGGLE_TOGGLE_STATE_PROPERTY, observation, source);
        let phrase = if state == ToggleState_On {
            "checked"
        } else if state == ToggleState_Off {
            "not checked"
        } else if state == ToggleState_Indeterminate {
            "partially checked"
        } else {
            "state changed"
        };
        crate::windows_speech::speak(phrase);
    }
}

fn emit(kind: AccessibilityEventKind, sequence: u64, sender: Ref<IUIAutomationElement>) {
    let Some(element) = sender.as_ref() else {
        return;
    };
    let observation = observe(element);
    sample_control_state(element, &observation, "event-sample");
    println!("{}", format_event_line(kind, sequence, &observation.snapshot));
}

fn emit_property(sequence: u64, property: PROPERTYID, sender: Ref<IUIAutomationElement>) {
    let Some(element) = sender.as_ref() else {
        return;
    };
    let observation = observe(element);
    emit_property_observation(sequence, property, &observation, "event");
}

#[implement(IUIAutomationFocusChangedEventHandler)]
struct FocusSink;
impl IUIAutomationFocusChangedEventHandler_Impl for FocusSink_Impl {
    fn HandleFocusChangedEvent(&self, sender: Ref<IUIAutomationElement>) -> Result<()> {
        if let Some(element) = sender.as_ref() {
            let observation = observe(element);
            emit_focus(&observation, "event");
            sample_control_state(element, &observation, "event-sample");
        }
        Ok(())
    }
}

#[implement(IUIAutomationEventHandler)]
struct EventSink;
impl IUIAutomationEventHandler_Impl for EventSink_Impl {
    fn HandleAutomationEvent(&self, sender: Ref<IUIAutomationElement>, eventid: EVENTID) -> Result<()> {
        if eventid == TEXT_CHANGED_EVENT {
            let n = TEXT_CHANGED_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
            emit(AccessibilityEventKind::TextChanged, n, sender);
        } else if eventid == TEXT_SELECTION_CHANGED_EVENT {
            let n = TEXT_SELECTION_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
            if let Some(element) = sender.as_ref() {
                crate::windows_textpattern2::inspect_selection(n, element);
            }
            emit(AccessibilityEventKind::TextSelectionChanged, n, sender);
        }
        Ok(())
    }
}

#[implement(IUIAutomationPropertyChangedEventHandler)]
struct PropertySink;
impl IUIAutomationPropertyChangedEventHandler_Impl for PropertySink_Impl {
    fn HandlePropertyChangedEvent(
        &self,
        sender: Ref<IUIAutomationElement>,
        propertyid: PROPERTYID,
        _newvalue: &VARIANT,
    ) -> Result<()> {
        let n = PROPERTY_CHANGED_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        emit_property(n, propertyid, sender);
        Ok(())
    }
}

pub fn run() -> Result<()> {
    let seconds = parse_monitor_seconds(env::args().nth(1).as_deref());
    unsafe {
        let _com = ComApartment::initialize()?;
        crate::windows_speech::initialize()?;

        let automation: IUIAutomation = match CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER) {
            Ok(value) => value,
            Err(_) => CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?,
        };
        let root = automation.GetRootElement()?;
        let cache = cache(&automation);
        let focus: IUIAutomationFocusChangedEventHandler = FocusSink.into();
        let events: IUIAutomationEventHandler = EventSink.into();
        let properties: IUIAutomationPropertyChangedEventHandler = PropertySink.into();

        automation.AddAutomationEventHandler(TEXT_CHANGED_EVENT, &root, TreeScope_Subtree, cache.as_ref(), &events).ok()?;
        automation.AddAutomationEventHandler(TEXT_SELECTION_CHANGED_EVENT, &root, TreeScope_Subtree, cache.as_ref(), &events).ok()?;
        automation.AddPropertyChangedEventHandlerNativeArray(
            &root,
            TreeScope_Subtree,
            cache.as_ref(),
            &properties,
            PROPERTY_CHANGE_PROPERTIES.as_ptr(),
            PROPERTY_CHANGE_PROPERTIES.len() as i32,
        ).ok()?;
        automation.AddFocusChangedEventHandler(cache.as_ref(), &focus).ok()?;

        println!("SCREEN_READER_RUNTIME_INIT = PASS");
        println!("UIA_INTERFACE = IUIAutomation");
        println!("EVENT_REGISTRATION = FOCUS_TEXT_SELECTION_PROPERTY_CHANGED");
        println!("UIA_FOCUS_POLL_FALLBACK = ENABLED");
        println!("UIA_STATE_POLL_FALLBACK = VALUE_TOGGLE_EVENT_SAMPLING");
        crate::windows_textpattern2::print_init_marker();
        println!("UIA_NATIVE_EVENTS_INIT = PASS");
        println!("MONITOR_SECONDS = {seconds}");
        println!("SCREEN_READER_PIPELINE = UIA_TO_SPEECH");

        let deadline = Instant::now() + Duration::from_secs(seconds);
        while Instant::now() < deadline {
            if let Ok(element) = automation.GetFocusedElement() {
                let observation = observe(&element);
                emit_focus(&observation, "poll");
                sample_control_state(&element, &observation, "poll");
            }
            thread::sleep(Duration::from_millis(50));
        }

        automation.RemoveFocusChangedEventHandler(&focus).ok()?;
        automation.RemovePropertyChangedEventHandler(&root, &properties).ok()?;
        automation.RemoveAutomationEventHandler(TEXT_SELECTION_CHANGED_EVENT, &root, &events).ok()?;
        automation.RemoveAutomationEventHandler(TEXT_CHANGED_EVENT, &root, &events).ok()?;

        println!(
            "COUNTS | focus={} | text_changed={} | text_selection={} | property_changed={}",
            FOCUS_COUNT.load(Ordering::Relaxed),
            TEXT_CHANGED_COUNT.load(Ordering::Relaxed),
            TEXT_SELECTION_COUNT.load(Ordering::Relaxed),
            PROPERTY_CHANGED_COUNT.load(Ordering::Relaxed)
        );
        println!(
            "FOCUS_SOURCE_COUNTS | event={} | poll={}",
            FOCUS_EVENT_COUNT.load(Ordering::Relaxed),
            FOCUS_POLL_COUNT.load(Ordering::Relaxed)
        );
        println!(
            "PROPERTY_SOURCE_COUNTS | poll={} | event_sample={}",
            PROPERTY_POLL_COUNT.load(Ordering::Relaxed),
            PROPERTY_EVENT_SAMPLE_COUNT.load(Ordering::Relaxed)
        );
        println!(
            "UIA_EVENT_CACHE_COUNTS | full_hit_events={} | fallback_properties={}",
            CACHE_FULL_HIT_EVENTS.load(Ordering::Relaxed),
            CACHE_FALLBACK_PROPERTIES.load(Ordering::Relaxed)
        );
        crate::windows_textpattern2::print_summary();
        crate::windows_speech::print_summary();
        println!("UIA_NATIVE_EVENTS_RUNTIME = PASS");
    }
    Ok(())
}
