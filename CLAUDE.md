# Burrow mobile

The Android app for Burrow, in Tauri 2 with Svelte 5. It is a client of `burrowd` over SSH: see "Mobile client" in Burrow's `docs/DESIGN.md` and "Over SSH" in its `docs/API.md`. The plan for v1 is milestone 20 there.

## Layout

- `crates/burrow-link`: everything that isn't Tauri: the phone's key, the pairing, the host key pin, the russh hop to `burrow proxy --role mobile`, and the bridge's allowlist of commands. It builds and tests on the desktop like any crate.
- `src-tauri`: wiring only, turning `burrow-link` into Tauri commands and events. `gen/android` is Tauri's generated Android project, with our manifest changes (no backups of app data).
- `src`: the Svelte 5 UI. `lib/` holds the API wrappers, types and stores, `screens/` one file per screen, `components/` the shared pieces.
- `burrow-api` comes from the Burrow repository by git, pinned to a commit.

## Checks

CI runs these. Run them before every commit:

```sh
pnpm check
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
.github/check-file-length
```

Source files stay under 300 lines (`.svelte` included), unless their first 10 lines say `modular-exempt: <reason>`.

## Environment

```sh
export JAVA_HOME=/usr/lib/jvm/java-21-openjdk
export ANDROID_HOME=$HOME/Android/Sdk
export NDK_HOME=$ANDROID_HOME/ndk/30.0.16248370
```

JDK 21, because the Android Gradle plugin doesn't take newer ones yet. The emulator is the AVD `burrow` (Android 37, x86_64). Run it headless, so it stays off the desktop, and drive it with `adb`:

```sh
$ANDROID_HOME/emulator/emulator -avd burrow -no-window -no-audio -no-boot-anim -no-snapshot-save -gpu swiftshader_indirect
cargo tauri android build --debug --apk --target x86_64
adb install -r src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
adb exec-out screencap -p > screen.png
```

To try shells there, give the AVD a PIN (`adb shell locksettings set-pin 1234`), type it at the unlock prompt with `adb shell input text 1234`, and type into a shell with `%s` for spaces; `adb shell input text` hands `;` and `$(…)` to the device's own shell. The prompt is a secure window, so a screenshot of it is black.

## Rules that shape the code

- **The key never leaves Rust.** The webview gets the public `authorized_keys` line, never the private key. App data is excluded from backups and device transfers.
- **The computer's host key is pinned** after the user confirms its fingerprint. A different key is refused and never re-pinned silently.
- **The daemon's text is untrusted.** Show it as Svelte text, never `{@html}`, and strip control characters.
- **Only the commands the screens need** cross from the webview, through the bridge's allowlist (`allow.rs`), not a generic `execute`. Shells are login shells only: `open_terminal` with `run` is refused.
- **Shells wait for the device unlock,** held by the bridge in Rust and asked for with `tauri-plugin-biometric`, which only Rust calls: the page has no permission for the plugin. Suspending the app locks shells again and ends their streams, so no shell output reaches a locked page.
- **The page types only into its own shells:** terminals this connection opened, and kept terminals a listing names, learned anew on each connection. xterm gets no clipboard or link addons, since the server's output goes straight into it.
- **Style:** as in Burrow: comments say why, in plain sentences; UI text in sentence case, with "…" on items that open a dialog.

## Workflow

Ask before pushing or opening a PR. Commit messages are an imperative subject, then prose saying what changed and why, with no conventional-commit prefixes.
