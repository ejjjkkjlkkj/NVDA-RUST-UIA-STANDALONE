# Multi-OS screen reader architecture

The repository is evolving from a Windows UIA prototype into a shared Rust screen-reader engine with native accessibility adapters per operating system.

## Shared Rust core

The portable core owns normalized accessibility events, element snapshots, navigation, text/caret/selection state, speech and braille command models, input routing, application profiles, configuration, diagnostics, and deterministic tests.

Platform adapters translate native APIs into the portable event model.

## Native adapters

| Platform | Native accessibility surface | Goal |
| --- | --- | --- |
| Windows | Microsoft UI Automation (`IUIAutomation6`, fallback `IUIAutomation`) | Full system screen reader |
| macOS | Accessibility API (`AXUIElement`, `AXObserver`) | Full desktop screen reader |
| Linux | AT-SPI2 over D-Bus | Full desktop screen reader |
| Android | `AccessibilityService`, `AccessibilityEvent`, `AccessibilityNodeInfo` | Full Android screen reader service |
| iOS/iPadOS | `UIAccessibility` and Accessibility framework | Accessible client/app integration; iOS does not expose a third-party system-wide replacement for VoiceOver |

## Event pipeline

```text
native OS event
    -> platform adapter
    -> normalized AccessibilityEventKind + ElementSnapshot
    -> event router
    -> navigation/text state
    -> speech + braille + diagnostics
```

The shared core must not import UIA, AX, AT-SPI2, Android framework or UIKit types.

## Validation policy

A platform is never marked implemented merely because it compiles. Evidence levels are tracked separately: portable unit tests; target compile/link; native API initialization; emulator/simulator runtime; real accessibility event capture; and physical-device/interactive assistive-technology validation.

Windows UIA is currently the reference functional backend. Android has an AccessibilityService probe and emulator test harness. macOS AX and Linux AT-SPI2 native adapters remain to be implemented. iOS has a UIKit accessibility probe and Simulator launch test, but VoiceOver validation requires physical Apple hardware.
