# CI validation levels

A platform is never called COMPLETE from compilation alone.

## Level 1 — portable core

- Rust formatting, clippy and unit tests pass.
- Shared event, semantic, navigation, speech and braille contracts compile.

## Level 2 — native SDK build

- Native platform adapter compiles against the current supported SDK/toolchain.
- Cross-target builds are checked where meaningful.

## Level 3 — virtual runtime

- Linux: graphical desktop/AT-SPI integration fixture.
- Android: Android 17/API 37 emulator with the AccessibilityService enabled and real accessibility events captured.
- iOS: current iOS Simulator app install/launch and accessibility-client probe. This does not prove VoiceOver system-level behavior.
- Windows/macOS hosted runners: non-interactive native smoke tests only when platform permissions/session semantics allow them.

## Level 4 — physical or interactive acceptance

Required before a native backend is marked COMPLETE:

- Windows: interactive Windows 11 self-hosted runner with UIA/MSAA/IA2 application fixtures.
- Linux: self-hosted GNOME Wayland desktop with AT-SPI2, keyboard routing, speech and braille fixtures.
- macOS: physical/self-hosted Mac with Accessibility permission granted and AXObserver/CGEvent acceptance tests.
- Android: physical current Pixel-class device with AccessibilityService, touch exploration, editable text, TTS and braille tests.
- iOS/iPadOS: physical device tests for VoiceOver interoperability of the app/client surface; iOS does not expose a third-party system-wide screen-reader replacement API.

Every CI result and release note must identify the highest level actually demonstrated. No emulator, simulator, compile-only or hosted-runner result may be reported as physical-device validation.
