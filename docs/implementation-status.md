# Snipe implementation status

Audit basis: `PLAN.md` v0.3 and the repository state at the time of this review.

How to close the gaps: [`repair-plan.md`](repair-plan.md). That document is the execution plan; this file remains the audit snapshot and must be updated when a repair-plan item is completed.

## Release decision

**NO-GO for a stable or feature-complete release.**

The repository is an early development preview. It contains useful foundations, but it does not satisfy the MVP, milestone acceptance criteria, or v1.0 Definition of Done in `PLAN.md`. A successful compile or installer build must not be interpreted as product completion.

The `windows 0.58` Credential Manager binding migration has been corrected in source, but the final Windows MSVC build must still pass in GitHub Actions. The local review machine has Rust installed but does not have Visual Studio C++ Build Tools or the Windows SDK import libraries.

## Milestone status

| Milestone | Status | Evidence and principal gaps |
|---|---|---|
| M0 | Partial | Workspace, Slint UI skeleton, Win32 notification window, GDI capture, Credential Manager wrapper, CI skeleton, ADRs, and a minimal Inno script exist. WGC/DXGI PoCs, measured performance/resource baselines, two-device mixed-DPI validation, complete license review, and several decisions in PLAN section 19 are absent. The UI report contains claims without attached measurements. |
| M1 | Partial | Single-instance, tray, hotkeys, display/window enumeration, GDI capture, clipboard, encoders, and basic settings code exist. The screenshot overlay now receives the captured frame, tracks physical desktop geometry, and uses per-monitor surfaces for mixed DPI. The mutex/pipe names are still not SID/session scoped, the pipe has no explicit current-user ACL, hotkey conflicts are only logged, saved hotkeys are not re-registered, settings do not control startup, and window selection/resize handles are absent. |
| M2 | Partial | Annotation now has an editor window/canvas/controller integration with rectangle, arrow, pen, mosaic, step, undo/redo, and original-resolution compositing. Selection, move/resize, delete/copy, crop, text editing/IME, and complete styling remain outside P0. |
| M3 | Partial | `PinManager` can create multiple basic Slint windows and supports copy/save/close. Window dragging, resizing, opacity control, click-through recovery, right-click menu, OCR/translation callbacks, cache policy, and the complete color picker workflow are absent. The color UI is a badge, not the planned pixel-grid magnifier and multi-format copy flow. |
| M4 | Partial | A `VisionProvider`, Chat Completions-style multimodal request, result window, timeout classification, and Credential Manager storage exist. Responses API support, retries, UI cancellation, request size limits/compression, long-image chunking, privacy/provider-origin display, contract tests, redirect-origin protection, and log redaction tests are absent. Credentials use one fixed target name instead of a normalized provider/origin-bound target. |
| M5 | Partial | Longshot hotkey/tray now opens a region-selection and manual capture workflow. It collects an initial frame plus user-triggered stable frames, applies 300-frame/30,000-px/five-minute limits, stitches, and retains a last-frame recovery result on failure. Automatic scrolling, fixed-header handling, richer preview, and the 30-sample corpus remain outside P0. |
| M6 | Partial | Upgrade IPC now uses a versioned allowlist protocol with `ready`, `busy`, `cancelled`, `incompatible`, and `rejected` replies. Active screenshot, longshot, annotation, and pin tasks return `busy`; the installer asks for graceful shutdown and fails visibly after bounded wait. Signing, RFC 3161 timestamping, license bundle, accessibility/i18n integration, diagnostics, install/upgrade/uninstall lifecycle tests, current-user pipe ACL, and Windows 10/11 clean-install evidence are still absent. |
| M7 | Not started | No beta evidence, frozen support matrix, signed release candidate, or formal release-gate evidence exists. |

## Current end-to-end blockers

### P0/P1 release blockers

1. The overlay does not display the captured frame and is not sized/positioned from virtual desktop physical coordinates. Region selection therefore is not a verified screenshot workflow, especially with negative monitor coordinates or mixed DPI.
2. Longshot has no acquisition workflow. The longshot hotkey and tray command call ordinary screenshot capture.
3. Annotation has no usable editor integration. The visible annotation action has no callback implementation.
4. Upgrade shutdown accepts an unauthenticated pipe command and exits immediately. There is no versioned handshake, active-task check, user confirmation, bounded installer wait, or required status response.
5. Single-instance and IPC objects are not scoped and secured as required. Pipe input is unbounded and has no read timeout; sender identity and current-user ACL are not enforced.
6. The capture backend is GDI-only. The planned WGC/DXGI primary paths, runtime capability detection, fallback matrix, protected-content behavior, and remote-session behavior are not implemented or tested.
7. Stable release gates have no evidence: 72-hour soak, 10,000 saves, at least 30 longshot samples with at least 90% success, mixed-DPI matrix, and clean Windows 10/11 install lifecycle tests.
8. Executable and installer signing, RFC 3161 timestamping, and third-party license publication are not implemented.

### Important functional gaps

- Settings save errors and Credential Manager failures are ignored while the UI reports success.
- Updating hotkeys in settings does not unregister/re-register them or report conflicts.
- The settings startup toggle does not modify HKCU Run; the installer task and application setting can disagree.
- Save directory/format settings and browse behavior are not wired in the settings UI.
- Pin opacity, always-on-top changes, click-through, OCR, and translation are not connected.
- AI supports only Chat Completions-style responses, despite the plan requiring both Responses and a verified compatible endpoint.
- AI retries are configured but not used, and cancellation tokens are not exposed to the UI.
- API keys are stored under one fixed credential target, not by normalized provider and origin.
- User-visible strings are embedded directly in Rust/Slint instead of using the existing i18n assets.
- Capture/save/AI errors are generally logged or shown as status text without the planned structured user-facing recovery.

## Build and release state

The following build-chain changes are present:

- `Cargo.lock` is generated and must be committed before tagging.
- CI and release jobs validate the committed lockfile and use `--locked`.
- CI runs formatting, Clippy with warnings denied, tests, and a release workspace build.
- `scripts/package.ps1` validates Tag/Cargo version consistency, requires the lockfile, requires Inno Setup, and fails if expected outputs are missing.
- Release packaging produces only the planned Inno Setup executable; no portable package is claimed.

Still required before the next Tag:

1. Run Windows CI and require all checks to pass. In particular, confirm the `windows 0.58` Credential Manager and GDI binding calls compile on `x86_64-pc-windows-msvc`.
2. Do not reuse a failed public Tag for a different artifact. Increment the prerelease/patch version after the fixes are merged.
3. Pin the Rust toolchain and Slint version after M0 validation. `slint = "1.9.0"` currently permits later 1.x releases; the lockfile currently determines the actual version.
4. Add installer smoke tests and artifact inspection before publishing a GitHub Release.
5. Keep releases marked prerelease until the blockers above and the applicable `PLAN.md` gates are closed.

## Verification performed in this review

Passed locally:

- `cargo fmt --all -- --check`
- `cargo metadata --locked --format-version 1 --no-deps`
- `git diff --check` (line-ending warnings only)
- Static comparison of affected calls with the downloaded `windows 0.58.0` generated bindings

Not completed locally:

- `cargo check`, Clippy, tests, and release build: blocked before project compilation because Visual Studio C++ Build Tools/Windows SDK are not installed (`link.exe` and Windows import libraries such as `kernel32.lib` are missing).
- Runtime screenshot, mixed-DPI, tray, installer, Credential Manager integration, and AI endpoint tests.

GitHub Actions on a Windows MSVC runner is the required next verification step.
