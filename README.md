# Burrow mobile

[Burrow](https://github.com/Edward-Pratt/burrow) on Android: the hosts on the computer running `burrowd`, with their alerts, live monitors and a week of history, on your phone.

The phone is a client of `burrowd`, not a copy of it. It signs in to that computer with SSH, as you, using a key it makes for itself, and that key can only run `burrow proxy --role mobile`. Secrets stay on the computer. [docs/HEADLESS.md](https://github.com/Edward-Pratt/burrow/blob/main/docs/HEADLESS.md) in the Burrow repository describes setting one up that stays on.

## Status

Milestone 20 of Burrow: work in progress. v1 covers pairing, hosts with alert badges, the monitor page with live samples and trends, alerts, and the agent-install prompt. Terminals and alerts while the app is closed come later.

## Building

It needs Rust 1.93 or newer with the `aarch64-linux-android` and `x86_64-linux-android` targets, `tauri-cli` 2, Node 24 with pnpm, JDK 21, and the Android SDK with the NDK. See `CLAUDE.md` for the environment and commands.

```sh
pnpm install
cargo tauri android build --apk --target aarch64
```

## License

GPL-3.0, like Burrow.
