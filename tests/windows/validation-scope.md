# Windows functional validation scope

The integrated keyboard-only CI validates the screen reader runtime as an external user-facing process rather than only testing internal Rust functions.

Validated scenarios include:

- WinForms edit, checkbox, combo box, and button interaction using keyboard input only;
- native Notepad editing and Shift+Arrow selection;
- UIA focus, text, selection, value, and toggle evidence;
- asynchronous Windows speech synthesis dispatch and flush;
- spoken checkbox state and spoken combo-box value changes, including `Detailed`;
- TextPattern2 caret and selection extraction where supported;
- TextPattern2 selection-delta speech for keyboard selection changes;
- password-field redaction and explicit secret-leak checks;
- an official pinned `nvaccess/nvda` source reference fetched read-only for comparison.

A PASS does not assert physical loudspeaker output, braille hardware, browse-mode navigation, Office support, or full screen-reader parity.
