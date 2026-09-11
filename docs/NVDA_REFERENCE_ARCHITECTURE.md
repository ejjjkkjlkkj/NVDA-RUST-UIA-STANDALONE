# NVDA reference architecture for the Rust screen reader

Reference snapshot: `nvaccess/nvda` official `master` at commit `c43cf6c1b0518326bd9ff0ed4712474feeb6131c` (2026-09-11).

This project is **not** an NVDA fork. NVDA is a read-only behavioral and architectural reference. No NVDA source is copied into the Rust runtime.

## What we adopt conceptually

### 1. Accessibility API backends must not drive user presentation directly

NVDA separates API handlers from NVDAObjects and output modules. Our equivalent target pipeline is:

`native backend (UIA/AX/AT-SPI/Android) -> AccessibleNode/TextRange -> event/command core -> presentation -> speech/braille`

The Windows UIA runtime must therefore map native UIA properties to platform-neutral semantic roles/states before presentation.

### 2. One abstract object model

NVDAObject exposes a uniform widget abstraction independent of MSAA/IA2/UIA/JAB and includes relationships such as parent, next, previous and first child. Our `AccessibleNode` is the start of the same idea, but needs stable identity, relations, actions and text access.

Required additions:

- stable opaque node identity, never plaintext-sensitive data;
- platform-neutral role/state mapping;
- parent / previous / next / first-child navigation;
- supported actions (focus, invoke, toggle, expand/collapse, set value);
- text provider abstraction for caret, selection and ranges;
- application-specific overlays/workarounds without contaminating the core.

### 3. Text ranges are a first-class subsystem

NVDA TextInfo abstracts text units, caret, selection and formatting. Our TextPattern2 experiment must evolve into a backend-neutral `TextRange` API supporting character, word, line, paragraph, document, caret and selection.

### 4. Input gestures and commands are separate from native keyboard hooks

NVDA normalizes keyboard, braille, touch and other input into gestures that bind to scripts/commands. We need a command registry before adding many hotkeys. A command must describe its scope, default gestures, help text and whether it is allowed on secure screens.

### 5. Speech is an output driver, not application logic

Presentation produces semantic utterances. Speech and braille consume them. Rapid navigation/selection can use replaceable speech while important focus/state announcements remain ordered.

### 6. Browse mode is a document model, not a collection of browser-specific key hacks

NVDA uses tree interceptors and browse mode to provide a linear representation of complex documents plus quick navigation for headings, links, tables, form fields, etc. Our web path must first build a document snapshot/linear index and only then add `H`, `K`, `T`, form-field navigation and focus/browse mode switching.

### 7. Application overlays are necessary

NVDA uses AppModules and overlay classes for broken/non-standard controls. We need backend-neutral quirk providers keyed by executable/framework/control signature. Workarounds must remain isolated and testable.

### 8. Security is part of the architecture

NVDA disables logging in secure mode because logs can expose credentials. Our stricter default remains: content redacted by default, password values never queried/spoken/logged, diagnostics plaintext only via explicit test opt-in, no elevation/UIAccess by default.

## Functional parity roadmap

Priority order is based on what makes a screen reader actually usable rather than on adding more event counters.

1. **Semantic UIA adapter**: UIA -> `AccessibleNode` -> `presentation::focus_utterance`.
2. **Command/input core**: screen-reader modifier, stop speech, repeat focus, object navigation commands.
3. **TextRange**: caret/selection/character/word/line and typing/deletion echo.
4. **Object navigation**: parent/first child/next/previous + activate/focus.
5. **Real web document model**: browse/focus mode, headings, links, controls, tables, elements list.
6. **Live regions / notifications**.
7. **Speech driver abstraction**, voice/rate/language switching and robust cancellation.
8. **Braille output/input abstraction**.
9. **Application overlays** for Terminal, browsers, Office and known UIA defects.
10. **Configuration/profile system**, portable mode and deterministic packaging.
11. **Windows secure-screen strategy** only after signing/install trust chain exists.
12. **Other OS backends** after the platform-neutral model is proven on Windows.

## Validation rule

A feature is not considered implemented merely because a UIA event appears in a log. It needs:

- a keyboard-only blind-user scenario;
- expected spoken semantic output;
- zero synthesized mouse input unless the tested feature explicitly requires pointer interaction;
- password/privacy regression checks;
- comparison against the pinned official NVDA behavior where meaningful;
- a CI artifact containing machine-readable evidence.
