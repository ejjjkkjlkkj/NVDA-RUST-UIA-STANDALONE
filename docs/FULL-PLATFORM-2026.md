# Complete platform plan — September 2026

This project targets a shared Rust screen-reader engine with native accessibility adapters per operating system. A backend is not considered complete until it covers accessible tree discovery, events, focus/navigation, text/caret/selection, actions, input routing, speech, braille, windows, web content, failure isolation, and real integration tests.

## Shared Rust core

The shared core must remain platform-neutral and own:

- normalized accessibility roles, states, relations and actions;
- normalized focus, text, caret, selection, value, live-region and window events;
- navigation model and review cursor;
- speech queue, priorities, cancellation and punctuation processing;
- braille cells, routing, cursor and input commands;
- keyboard command maps and gesture command maps;
- application/browser profiles;
- event coalescing, deduplication and latency control;
- logging, crash isolation and diagnostics;
- deterministic unit and replay tests.

Platform adapters translate native objects/events into this model. Native object handles must not leak into the shared core.

## Windows — complete backend target

Primary API: Microsoft UI Automation using the current windows-rs generation.

Required coverage:

- CUIAutomation8 / IUIAutomation6 when available, with compatibility fallback;
- handler groups rather than independent handlers where supported;
- focus, property, structure, notification, window, async-content, text-edit and active-text-position events;
- UIA tree navigation and CacheRequest-based bulk property retrieval;
- Invoke, Toggle, Selection, SelectionItem, ExpandCollapse, RangeValue, Value, Scroll, Grid, GridItem, Table, TableItem, Window, Transform, VirtualizedItem and ItemContainer patterns;
- Text, Text2/TextRange, TextEdit, TextChild, annotations and embedded objects;
- LegacyIAccessible pattern and direct MSAA/IAccessible compatibility path for legacy applications;
- IAccessible2 compatibility module for browser/document cases where IA2 remains richer than UIA;
- global keyboard/input routing, mouse review and hit testing;
- Windows.Media.SpeechSynthesis plus pluggable third-party synthesizers;
- HID braille plus pluggable legacy braille drivers and Liblouis translation;
- x64 primary build and ARM64 compile/runtime validation.

Real tests should exercise Windows Terminal/PowerShell, Notepad, Settings, Explorer, Win32 controls, WPF, WinUI/XAML, Chromium and Firefox. Hosted CI can compile and run deterministic UIA tests; full desktop interaction should additionally run on an interactive self-hosted Windows 11 runner.

## Linux — complete backend target

Primary API: AT-SPI2 over D-Bus.

2026 baseline:

- libatspi API 2.0, current 2.61.x development line;
- stable compatibility baseline at least at-spi2-core 2.58.6 because current Orca requires it;
- Rust `atspi` 0.30.x / `atspi-connection` 0.14.x generation.

Required coverage:

- desktop/application/accessible tree and cache;
- object events, state/property changes and focus;
- Action, Component, Collection, Document, EditableText, Hyperlink, Hypertext, Image, Selection, Table, TableCell, Text and Value interfaces;
- caret, text attributes, text selections, geometry and point hit testing;
- modern Atspi.Device key grabs/watchers for Wayland-capable input handling; do not build new code around the legacy DeviceEventListener D-Bus interface;
- X11 fallback where required;
- Speech Dispatcher 0.12.1 as the default Linux speech transport, with direct synth plugins optional;
- BRLTTY/BrlAPI for braille devices and Liblouis for translation;
- GTK, Qt, Chromium/Electron and Firefox coverage;
- GNOME Wayland as the reference desktop, with KDE validation as a second target.

Hosted Linux CI should run a real AT-SPI bus and accessible GTK/Qt fixture under a graphical session, not only unit tests. A self-hosted GNOME Wayland runner is required for full key-grab, compositor and hardware braille coverage.

## macOS — complete backend target

Primary API: Apple ApplicationServices Accessibility API.

Required coverage:

- AXUIElement system-wide and per-process elements;
- AXObserver notifications and robust observer lifecycle;
- complete attribute/action discovery rather than hard-coded role subsets;
- AXTextMarker / AXTextMarkerRange and parameterized text attributes;
- focus, selected text, caret, value, title, description, role/subrole, children, windows and geometry;
- process trust via AXIsProcessTrustedWithOptions and clear permission diagnostics;
- CGEvent event taps for keyboard/mouse command routing when permitted;
- AVSpeechSynthesizer for system speech plus a pluggable synthesis layer;
- Apple braille translation APIs where useful and an independent HID/driver layer for screen-reader-owned displays;
- AppKit, SwiftUI, Catalyst, Safari/WebKit, Chromium/Electron and terminal coverage.

Rust implementation should prefer a maintained safe AX wrapper where it has complete coverage and fall back to direct ApplicationServices bindings for missing symbols. Current candidates include `axuielement` 0.9.x and `objc2-application-services` 0.3.x.

GitHub macOS runners are useful for compile/tests and Simulator work, but a complete system-wide AX test requires a physical/self-hosted Mac with Accessibility permission granted to the runner process. Test Finder, Safari, TextEdit, Terminal and System Settings there.

## Android — complete screen-reader backend target

Target current Android 17 / API 37, not a minimal demo service.

2026 toolchain baseline:

- Android 17 / API 37;
- Android Studio Quail 4 (2026.1.4) stable;
- Android Gradle Plugin 9.4.0;
- Gradle 9.6;
- JDK 17;
- NDK 28.2.13676358;
- Platform Tools 37.0.1;
- Android Emulator 36.6.11 or newer;
- API 37 emulator image revision 5 or newer.

Architecture:

- thin Kotlin/Java Android shell owns AccessibilityService lifecycle and framework callbacks;
- Rust shared core owns event normalization, navigation policy, speech/braille policy and commands;
- a narrow JNI bridge transfers immutable event snapshots and command results;
- no reflection against private Android fields; Android 17 strengthens restrictions around static-final mutation and private implementation assumptions.

Required AccessibilityService coverage:

- all relevant AccessibilityEvent types;
- AccessibilityWindowInfo multi-window model and AccessibilityNodeInfo trees;
- accessibility focus, input focus, actions and traversal;
- text changes including Android 17 text-change-type/composition information;
- editable text, selections and accessibility InputMethod support;
- FLAG_RETRIEVE_INTERACTIVE_WINDOWS, FLAG_REPORT_VIEW_IDS and include-not-important-view handling where appropriate;
- touch exploration using TouchInteractionController;
- gesture recognition and dispatchGesture;
- key filtering and physical keyboard commands;
- magnification controller integration;
- accessibility button integration without relying on it as the sole activation path;
- screenshot APIs for optional visual fallback, respecting Android security/privacy restrictions;
- overlays where needed for focus/highlight UI;
- TextToSpeech with queue/cancel/rate/pitch/locale control;
- BrailleDisplayController HID USB/Bluetooth support (API 35+) plus braille translation and routing;
- phones, tablets, foldables, Android TV, Wear OS and physical keyboard paths where the framework supports them;
- Chrome/WebView and Compose accessibility validation.

Android tests must run on both Android 17 emulator and at least one physical Pixel. Emulator tests can enable the service with adb, drive Settings/test fixtures, verify accessibility focus and inspect logcat. Physical-device tests are needed for real touch exploration, Bluetooth/USB braille, audio latency and OEM differences.

## iOS / iPadOS — supported client, not a system-wide replacement

The shared Rust core may compile for iOS and an iOS app can expose excellent accessibility, speech and braille-aware UI. However, a normal third-party iOS application cannot replace VoiceOver system-wide or inspect/control arbitrary other apps like a macOS AX accessibility client can.

Therefore iOS scope is:

- shared-core compilation;
- accessible UIKit/SwiftUI application UI;
- AVSpeechSynthesizer and custom speech-provider research;
- VoiceOver compatibility tests on physical iPhone/iPad;
- Simulator build/launch tests for application integration.

Do not mark iOS as a full system screen-reader backend unless Apple exposes a public entitlement/API that changes this platform limitation.

## CI and real-test matrix

### GitHub-hosted continuous validation

- Windows Server 2025 / VS 2026 x64: Rust build, unit tests, Windows adapter compile/integration fixtures.
- Windows 11 ARM64 / VS 2026: ARM64 compile and portable-core tests where available.
- Ubuntu 24.04 GA: stable Linux integration baseline.
- Ubuntu 26.04 preview: forward compatibility.
- macOS 26 ARM64: stable Apple-hosted build/test baseline.
- Xcode 27 public-preview runner: forward Apple SDK compatibility.
- Android 17 API 37 emulator: instrumented AccessibilityService tests.
- iOS Simulator: app build/install/launch and app-level accessibility probes.
- Rust stable is required; beta/nightly are advisory compatibility jobs.

### Physical/self-hosted acceptance lane

A release is only "full platform validated" when the following pass:

- Windows 11 interactive desktop runner: UIA/MSAA/IA2 + real apps + real TTS/braille if hardware is attached.
- GNOME Wayland Linux runner: AT-SPI tree/events + key grabs + Speech Dispatcher + BRLTTY.
- physical Mac: trusted AX client + AXObserver + CGEvent tap + Safari/TextEdit/Finder + speech.
- physical Android 17 device: AccessibilityService + touch exploration + gestures + IME + Chrome + speech + HID braille when hardware is present.
- physical iPhone/iPad: VoiceOver interoperability tests only; not system-wide replacement testing.

## Definition of complete

A platform must not be labelled COMPLETE merely because it compiles. COMPLETE requires:

1. native accessibility tree and event ingestion;
2. semantic role/state/relation normalization;
3. focus and object navigation;
4. text, caret and selection navigation;
5. actionable controls and editable text;
6. global command/input path appropriate to the OS;
7. low-latency cancellable speech;
8. braille output/input and translation path;
9. browser/document coverage;
10. resilience to disappearing/hung accessible objects;
11. deterministic tests plus native integration tests;
12. at least one real-device/self-hosted acceptance run for OS features that hosted CI cannot faithfully exercise.
