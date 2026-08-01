# grxxt

[![CI](https://github.com/sxndmxn/grxxt/actions/workflows/ci.yml/badge.svg)](https://github.com/sxndmxn/grxxt/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/grxxt.svg)](https://crates.io/crates/grxxt)
[![license](https://img.shields.io/badge/license-MIT-f1c35f.svg)](LICENSE)

A brutalist TUI greeter for [greetd](https://sr.ht/~kennylevinsen/greetd/).
It runs directly on a TTY: no display server or desktop shell required.

![grxxt login screen](assets/grxxt.png)

## Features

- Focused username/password login flow with clear error states
- Optional avatar with automatic terminal graphics detection and a half-block fallback
- Clock, date, and keyboard-driven shutdown, reboot, and suspend controls
- TOML configuration for the session command, avatar, and color theme
- The common single-secret PAM authentication flow

Additional visible or multi-secret authentication challenges are rejected cleanly; grxxt does not guess answers to PAM prompts.

## Install

Install the binary from crates.io:

```sh
cargo install grxxt
```

For a system greetd setup, clone the repository and run the installer:

```sh
cargo build --release
./install.sh
```

The installer places the binary in `/usr/local/bin` and example configuration in `/etc/greetd`. Existing greetd configuration is preserved.

Then enable greetd:

```sh
sudo systemctl enable greetd
```

## Configuration

`/etc/greetd/grxxt.toml`:

```toml
session = "/usr/bin/Hyprland"
# avatar = "/path/to/avatar.png"

[theme]
background = "#0b0a13"
foreground = "#f6f1e3"
accent = "#f1c35f"
error = "#d14b64"
```

All fields are optional. A missing config uses these defaults; malformed configuration is reported instead of silently starting a different session.

Configure greetd to start grxxt as its greeter:

```toml
[terminal]
vt = 1

[default_session]
command = "/usr/local/bin/grxxt"
user = "greeter"
```

Power controls call `systemctl`, so they require systemd and the appropriate policy permissions.

## Key bindings

| Key | Action |
|---|---|
| Tab / Shift+Tab | Switch fields |
| Enter | Move to the password field or submit |
| F1 | Shut down |
| F2 | Reboot |
| F3 | Suspend |
| Esc | Quit a debug build |

## Development

The same checks run locally and in CI:

```sh
cargo fmt --all -- --check
cargo test --locked --all-targets --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo package --locked
```

## License

[MIT](LICENSE)
