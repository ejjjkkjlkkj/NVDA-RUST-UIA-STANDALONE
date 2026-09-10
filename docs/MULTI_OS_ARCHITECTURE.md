# Multi-OS screen reader architecture

## Goal

Build one Rust screen-reader core with native accessibility backends per operating system.

The shared core must not depend on COM, AXUIElement, D-Bus, a particular speech engine, or a particular braille API.

## Platform backends

### Windows

Native API: Microsoft UI Automation (UIA).

Current status: functional reference backend.

Responsibilities:

- focus and structure events;
- text/caret/selection events;
- accessible tree traversal;
- control patterns and actions;
- application/framework metadata.

### macOS

Native API: macOS Accessibility API (`AXUIElement`, `AXObserver`).

Target responsibilities:

- system-wide focused element;
- AX attributes, roles, values and actions;
- AX notifications through `AXObserver`;
- text markers, caret and selections;
- Accessibility permission/trust detection.

### Linux

Native API: AT-SPI2 over D-Bus.

Target responsibilities:

- accessibility bus connection;
- accessible tree and application roots;
- focus/object/window events;
- Text and EditableText interfaces;
- caret, selection, role, state and actions;
- Wayland/X11 independence at the screen-reader core level.

## Shared Rust core

The portable core owns normalized data structures and policies:

```text
Native backend
    -> normalized AccessibilityEvent
    -> event router
    -> navigation / virtual buffer
    -> speech queue
    -> braille queue
    -> user commands / actions
```

The core should contain:

- normalized roles and states;
- element snapshots;
- focus model;
- text model;
- navigation commands;
- event coalescing/debouncing;
- speech priority and cancellation;
- braille presentation model;
- configuration and profiles;
- test fixtures independent from native APIs.

## Backend contract

`src/platform.rs` defines the initial portable backend contract.

Native backends are selected at compile time:

- `cfg(target_os = "windows")` -> UIA;
- `cfg(target_os = "macos")` -> AX;
- `cfg(target_os = "linux")` -> AT-SPI2.

The contract will evolve from the current monitor-style `run` entry point toward event producers and command/action consumers.

## CI strategy

Stable Rust is mandatory on Windows, Linux and macOS.

Windows is the authoritative runtime platform for the current UIA implementation.
Linux and macOS validate the shared core immediately and will become authoritative for their native backends as those implementations land.

Beta and nightly jobs are compatibility canaries and must not define the minimum supported Rust version until explicitly promoted.

## Implementation order

1. Stabilize portable event/element types.
2. Refactor Windows UIA backend behind the common contract.
3. Add Linux AT-SPI2 connection + focus/text event prototype.
4. Add macOS AXUIElement + AXObserver focus/text event prototype.
5. Normalize roles/states/actions across all three APIs.
6. Add shared navigation engine.
7. Add speech abstraction and native engines.
8. Add braille abstraction.
9. Add application profiles and virtual document model.
10. Add end-to-end accessibility regression suites per OS.

## Non-goals

The project must not force one operating system's accessibility object model onto the other platforms. UIA IDs, AX attributes and AT-SPI interface names stay inside their adapters; only normalized semantics cross into the shared core.
