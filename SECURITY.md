# Security policy

This project is an assistive-technology runtime. A screen reader necessarily observes accessibility metadata and may process text that is private. Security therefore starts with minimizing privilege, minimizing retention, and treating all application-provided accessibility data as untrusted.

## Security invariants

The following are design requirements, not optional recommendations:

- **No elevation by default.** The normal executable must run as the interactive user. `requireAdministrator`, `highestAvailable`, and `uiAccess="true"` are rejected by CI.
- **UIAccess is disabled until a trusted distribution chain exists.** Enabling Windows UIAccess will require Authenticode signing, a protected installation location, a reviewed manifest, and dedicated tests. Development builds must not silently acquire UIAccess.
- **No plaintext diagnostic content by default.** Spoken text, selected text, UIA `Name`, UIA `AutomationId`, and control values are redacted from diagnostic output unless `NVDA_RUST_DIAGNOSTICS_INCLUDE_TEXT=1` is explicitly supplied for controlled test evidence.
- **Passwords remain redacted even in test-evidence mode.** `IsPassword` is checked before ValuePattern or TextPattern2 text retrieval. Test-mode diagnostics are never allowed to override password protection.
- **No telemetry or network reporting in the runtime by default.** Runtime accessibility and speech processing are local. Network access used by build tooling is separate from the screen-reader runtime.
- **Bounded in-memory state.** Focus, control-state, and selection tracking use bounded collections. Sensitive text is kept only as needed for current accessibility behavior and is not intentionally persisted by the runtime.
- **Untrusted UIA data is data, not code.** Accessible names, values, selected text, automation IDs, notification strings, and document content must never be interpreted as commands, paths, shell fragments, or format strings.
- **Least-privilege CI.** Test/build jobs use read-only repository permissions. Release publication is a separate job with only the permissions required to publish and create provenance attestations.
- **Immutable critical Actions.** Security, release, and Windows E2E workflows pin third-party GitHub Actions to full commit SHAs.
- **Locked Rust graph.** CI uses `Cargo.lock`, checks the exact Microsoft `windows-rs` revision, runs RustSec auditing, and records dependency evidence.
- **Release integrity.** Release payloads include SHA-256 checksums, source-commit information, toolchain information, and GitHub/Sigstore build provenance.

## Diagnostic modes

### Default mode

Do not set `NVDA_RUST_DIAGNOSTICS_INCLUDE_TEXT`.

Expected marker:

```text
DIAGNOSTIC_TEXT_POLICY = REDACTED_DEFAULT
PASSWORD_TEXT_POLICY = ALWAYS_REDACTED
```

Content-bearing fields are represented by metadata such as `<redacted chars=17>`.

### Controlled CI evidence mode

`NVDA_RUST_DIAGNOSTICS_INCLUDE_TEXT=1` may be used only when an automated test needs exact speech or accessibility evidence. Artifacts from such runs can contain ordinary non-password UI text and must be treated accordingly.

Password controls stay source-redacted in this mode and have dedicated leak tests.

## Windows UIAccess policy

Microsoft UIAccess allows approved assistive applications to interact across additional integrity boundaries. It also increases the security impact of a compromised process.

This repository therefore keeps UIAccess disabled until all of the following exist together:

1. an Authenticode code-signing process whose private key is not exposed to ordinary build jobs;
2. installation into a Windows protected location such as Program Files;
3. a dedicated, reviewed application manifest;
4. release provenance and checksum verification;
5. tests proving password redaction, secure-desktop behavior, and expected integrity-level boundaries;
6. a recovery/update design that cannot silently replace the signed executable with an untrusted binary.

Until then, inability to read an elevated application is preferable to silently weakening the workstation security boundary.

## Supply-chain checks

The security workflow validates at least:

- Rust formatting and Clippy warnings as errors;
- locked builds;
- RustSec vulnerability advisories;
- exact `windows-rs` commit pinning;
- absence of accidental elevation/UIAccess declarations;
- SHA-pinned critical GitHub Actions;
- Windows PE ASLR / high-entropy VA / NX compatibility bits;
- default diagnostic redaction;
- password privacy while CI evidence mode is deliberately enabled.

## Release trust

A checksum proves that a downloaded file matches a named digest. A provenance attestation proves information about where/how an artifact was built. Neither alone proves that software is vulnerability-free. Both are required layers, alongside source review and automated security tests.

## Reporting a vulnerability

If GitHub's **Report a vulnerability** control is available for this repository, use it for security-sensitive reports. Do not post passwords, private documents, signing material, exploit samples containing personal data, or other secrets in a public issue.

If private vulnerability reporting is unavailable, open a public issue containing only a minimal non-sensitive description that a private security contact is needed. Do not include exploit details or confidential data there.
