# Functional gap matrix against official NVDA reference

Pinned NVDA: `c43cf6c1b0518326bd9ff0ed4712474feeb6131c`.

| Capability | NVDA reference concept | Current Rust status | Next implementation |
|---|---|---|---|
| API abstraction | API handlers + NVDAObject | Partial | Make UIA produce `AccessibleNode` before speech |
| Semantic roles/states | controlTypes/NVDAObject | Partial | Canonical UIA ControlType -> Role + states |
| Focus presentation | speech from semantic object | Partial/direct UIA strings | Route through `presentation::focus_utterance` |
| Object navigation | parent/next/previous/firstChild | Missing | Add relation/action abstraction and UIA TreeWalker adapter |
| Text model | TextInfo | Experimental TextPattern2 only | Backend-neutral TextRange |
| Input commands | InputGesture + scripts | Missing | Command registry + keyboard gesture layer |
| Stop/pause speech | global command | Missing | Immediate cancel/pause commands |
| Browse mode | TreeInterceptor/BrowseMode | Missing | Linear document model + browse/focus mode |
| Quick navigation | heading/link/table/form navigation | Missing | Indexed semantic document |
| Live regions | accessibility events | Missing | UIA LiveRegionChanged/Notification |
| App-specific fixes | AppModules/overlays | Missing | Quirk/overlay registry |
| Speech drivers | SynthDriver | Windows SpeechSynthesizer only | Output-driver trait + settings |
| Braille | braille drivers + input | Missing | Braille output/input trait |
| Configuration | config profiles | Missing | persistent typed config/profile system |
| Secure mode | restricted logging/navigation | Strong partial | preserve redaction; design signed secure-screen path |
| Packaging | installer/portable | Test ZIP only | signed installer + portable package later |

The first milestone is not “more UIA events”. It is a correct semantic pipeline that can support all later features without duplicating platform-specific logic.
