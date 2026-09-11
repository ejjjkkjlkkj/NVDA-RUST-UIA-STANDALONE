use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::{
    env, thread,
    time::{Duration, Instant},
};

use nvda_rust_uia_standalone::{
    AccessibilityEventKind, ElementSnapshot, format_event_line, parse_monitor_seconds,
    presentation::focus_utterance,
    semantic::{AccessibleNode, Role},
    uia_semantic::{
        UIA_COMBO_BOX_CONTROL_TYPE_ID, UiaSemanticInput, build_accessible_node,
        role_from_control_type_id,
    },
};
use windows::Win32::*;
use windows_core::{BSTR, Ref, Result, implement};

const TEXT_SELECTION_CHANGED_EVENT: EVENTID = EVENTID(20014);
const TEXT_CHANGED_EVENT: EVENTID = EVENTID(20015);
const VALUE_PATTERN: PATTERNID = PATTERNID(10002);
const TOGGLE_PATTERN: PATTERNID = PATTERNID(10015);
const PROCESS_ID_PROPERTY: PROPERTYID = PROPERTYID(30002);
const CONTROL_TYPE_PROPERTY: PROPERTYID = PROPERTYID(30003);
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
static SEMANTIC_FOCUS_COUNT: AtomicU64 = AtomicU64::new(0);
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
    control_type_id: i32,
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

fn diagnostic_snapshot(snapshot: &ElementSnapshot) -> ElementSnapshot {
    ElementSnapshot {
        process_id: snapshot.process_id,
        framework: snapshot.framework.clone(),
        class_name: snapshot.class_name.clone(),
        role: snapshot.role.clone(),
        name: crate::windows_diagnostics::text(&snapshot.name),
        automation_id: crate::windows_diagnostics::text(&snapshot.automation_id),
    }
}

fn cache(automation: &IUIAutomation) -> Option<IUIAutomationCacheRequest> {
    let result = unsafe {
        let cache = automation.CreateCacheRequest().ok()?;
        for property in [
            PROCESS_ID_PROPERTY,
            CONTROL_TYPE_PROPERTY,
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

        let process_id = match element.CachedProcessId() {
            Ok(value) => value,
            Err(_) => {
                fallback += 1;
                element.CurrentProcessId().unwrap_or_default()
            }
        };
        let control_type_id = match element.CachedControlType() {
            Ok(value) => value.0,
            Err(_) => {
                fallback += 1;
                element
                    .CurrentControlType()
                    .map(|value| value.0)
                    .unwrap_or_default()
            }
        };
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
                element.CurrentIsPassword().map(Into::into).unwrap_or(false)
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
            control_type_id,
        }
    }
}

fn focus_identity(observation: &Observation) -> String {
    let s = &observation.snapshot;
    let process_id = s.process_id.to_string();
    crate::windows_diagnostics::identity_key(&[
        &process_id,
        &s.framework,
        &s.class_name,
        &s.role,
        &s.name,
        &s.automation_id,
    ])
}

fn semantic_node(element: &IUIAutomationElement, observation: &Observation) -> AccessibleNode {
    let role = role_from_control_type_id(observation.control_type_id);
    let value = if observation.is_password || !role.reports_value_on_focus() {
        String::new()
    } else {
        unsafe {
            element
                .GetCurrentPatternAs::<IUIAutomationValuePattern>(VALUE_PATTERN)
                .ok()
                .and_then(|pattern| pattern.CurrentValue().ok())
                .map(|value| value.display().to_string())
                .unwrap_or_default()
        }
    };

    let toggle_state = unsafe {
        element
            .GetCurrentPatternAs::<IUIAutomationTogglePattern>(TOGGLE_PATTERN)
            .ok()
            .and_then(|pattern| pattern.CurrentToggleState().ok())
            .map(|state| {
                if state == ToggleState_On {
                    1
                } else if state == ToggleState_Indeterminate {
                    2
                } else {
                    0
                }
            })
    };

    let is_enabled = unsafe {
        element
            .CurrentIsEnabled()
            .map(Into::into)
            .unwrap_or(true)
    };
    let is_keyboard_focusable = unsafe {
        element
            .CurrentIsKeyboardFocusable()
            .map(Into::into)
            .unwrap_or(false)
    };
    let has_keyboard_focus = unsafe {
        element
            .CurrentHasKeyboardFocus()
            .map(Into::into)
            .unwrap_or(false)
    };
    let is_offscreen = unsafe {
        element
            .CurrentIsOffscreen()
            .map(Into::into)
            .unwrap_or(false)
    };

    let identity = focus_identity(observation);
    let mut node = build_accessible_node(UiaSemanticInput {
        process_id: observation.snapshot.process_id,
        platform_id: &identity,
        control_type_id: observation.control_type_id,
        native_role: &observation.snapshot.role,
        name: &observation.snapshot.name,
        description: "",
        value: &value,
        is_password: observation.is_password,
        is_enabled,
        is_keyboard_focusable,
        has_keyboard_focus,
        is_offscreen,
        is_selected: None,
        toggle_state,
        expand_collapse_state: None,
    });

    // First isolated overlay: Windows Terminal exposes a document-like UIA
    // control but users need a terminal semantic. Keep the quirk outside the
    // platform-neutral presentation layer.
    if observation.snapshot.class_name == "TermControl" {
        node.role = Role::Terminal;
    }

    node
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

fn speak_focus(element: &IUIAutomationElement, observation: &Observation) {
    let node = semantic_node(element, observation);
    let semantic_sequence = SEMANTIC_FOCUS_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    println!(
        "SEMANTIC_FOCUS #{semantic_sequence} | role={} | password={} | value_present={}",
        node.role.canonical_name(),
        observation.is_password,
        !node.safe_value().is_empty()
    );

    if let Some(utterance) = focus_utterance(&node) {
        crate::windows_speech::speak(&utterance.text);
    }
}

fn emit_focus(element: &IUIAutomationElement, observation: &Observation, source: &str) {
    if !focus_changed(observation) {
        return;
    }

    let sequence = FOCUS_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if source == "event" {
        FOCUS_EVENT_COUNT.fetch_add(1, Ordering::Relaxed);
    } else {
        FOCUS_POLL_COUNT.fetch_add(1, Ordering::Relaxed);
    }

    let s = diagnostic_snapshot(&observation.snapshot);
    println!(
        "FOCUS #{sequence} | Source={source} | PID={} | Framework={} | Class={} | Role={} | Name={} | AutomationId={}",
        s.process_id, s.framework, s.class_name, s.role, s.name, s.automation_id
    );
    speak_focus(element, observation);
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
    let s = diagnostic_snapshot(&observation.snapshot);
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
    let is_combo_box = observation.control_type_id == UIA_COMBO_BOX_CONTROL_TYPE_ID;

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
                println!(
                    "VALUE_SPEECH #{sequence} | control=combo-box | value={}",
                    crate::windows_diagnostics::text(phrase)
                );
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
    let safe_snapshot = diagnostic_snapshot(&observation.snapshot);
    println!("{}", format_event_line(kind, sequence, &safe_snapshot));
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
            emit_focus(element, &observation, "event");
            sample_control_state(element, &observation, "event-sample");
        }
        Ok(())
    }
}

#[implement(IUIAutomationEventHandler)]
struct EventSink;
impl IUIAutomationEventHandler_Impl for EventSink_Impl {
    fn HandleAutomationEvent(
        &self,
        sender: Ref<IUIAutomationElement>,
        eventid: EVENTID,
    ) -> Result<()> {
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

        let automation: IUIAutomation =
            match CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER) {
                Ok(value) => value,
                Err(_) => CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?,
            };
        let root = automation.GetRootElement()?;
        let cache = cache(&automation);
        let focus: IUIAutomationFocusChangedEventHandler = FocusSink.into();
        let events: IUIAutomationEventHandler = EventSink.into();
        let properties: IUIAutomationPropertyChangedEventHandler = PropertySink.into();

        automation
            .AddAutomationEventHandler(
                TEXT_CHANGED_EVENT,
                &root,
                TreeScope_Subtree,
                cache.as_ref(),
                &events,
            )
            .ok()?;
        automation
            .AddAutomationEventHandler(
                TEXT_SELECTION_CHANGED_EVENT,
                &root,
                TreeScope_Subtree,
                cache.as_ref(),
                &events,
            )
            .ok()?;
        automation
            .AddPropertyChangedEventHandlerNativeArray(
                &root,
                TreeScope_Subtree,
                cache.as_ref(),
                &properties,
                PROPERTY_CHANGE_PROPERTIES.as_ptr(),
                PROPERTY_CHANGE_PROPERTIES.len() as i32,
            )
            .ok()?;
        automation
            .AddFocusChangedEventHandler(cache.as_ref(), &focus)
            .ok()?;

        println!("SCREEN_READER_RUNTIME_INIT = PASS");
        println!("SCREEN_READER_RUNTIME_VERSION = V3_SEMANTIC");
        println!("UIA_INTERFACE = IUIAutomation");
        println!("UIA_SEMANTIC_ADAPTER = ENABLED");
        println!("EVENT_REGISTRATION = FOCUS_TEXT_SELECTION_PROPERTY_CHANGED");
        println!("UIA_FOCUS_POLL_FALLBACK = ENABLED");
        println!("UIA_STATE_POLL_FALLBACK = VALUE_TOGGLE_EVENT_SAMPLING");
        crate::windows_textpattern2::print_init_marker();
        println!("UIA_NATIVE_EVENTS_INIT = PASS");
        println!("MONITOR_SECONDS = {seconds}");
        // Compatibility marker consumed by existing E2E assertions.
        println!("SCREEN_READER_PIPELINE = UIA_TO_SPEECH");
        println!(
            "SCREEN_READER_SEMANTIC_PIPELINE = UIA_TO_ACCESSIBLE_NODE_TO_PRESENTATION_TO_SPEECH"
        );

        let deadline = Instant::now() + Duration::from_secs(seconds);
        while Instant::now() < deadline {
            if let Ok(element) = automation.GetFocusedElement() {
                let observation = observe(&element);
                emit_focus(&element, &observation, "poll");
                sample_control_state(&element, &observation, "poll");
            }
            thread::sleep(Duration::from_millis(50));
        }

        automation.RemoveFocusChangedEventHandler(&focus).ok()?;
        automation
            .RemovePropertyChangedEventHandler(&root, &properties)
            .ok()?;
        automation
            .RemoveAutomationEventHandler(TEXT_SELECTION_CHANGED_EVENT, &root, &events)
            .ok()?;
        automation
            .RemoveAutomationEventHandler(TEXT_CHANGED_EVENT, &root, &events)
            .ok()?;

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
            "SEMANTIC_COUNTS | focus_presentations={}",
            SEMANTIC_FOCUS_COUNT.load(Ordering::Relaxed)
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
