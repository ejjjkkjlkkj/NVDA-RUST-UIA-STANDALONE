# Official NVDA sources consulted

All references below are from official `nvaccess/nvda` commit `c43cf6c1b0518326bd9ff0ed4712474feeb6131c` and are used read-only for behavioral/architectural comparison.

- `projectDocs/design/technicalDesignOverview.md`: component boundaries, API handlers, NVDAObject, TextInfo, input gestures, output drivers, app modules, tree interceptors and browse mode.
- `source/NVDAObjects/__init__.py`: uniform widget abstraction, API classes, overlays, relationships, actions and text access.
- `source/browseMode.py`: browse/focus mode and quick navigation architecture.
- `source/inputCore.py`: platform-independent input gesture/command concepts.
- `user_docs/en/userGuide.md`: user-visible behavior and command expectations.

Policy: concepts and externally observable behavior may guide this project; NVDA source code is not copied into the Rust runtime.
