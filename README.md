# grxxt

A brutalist TUI greeter for [greetd](https://sr.ht/~kennylevinsen/greetd/).

Runs directly on the TTY. No GPU, no Wayland, no X11 — just a login prompt.

## Features

- Centered login form with avatar, username, and password fields
- Clock display (HH:MM + date)
- Power controls: shutdown (F1), reboot (F2), suspend (F3)
- TOML-based configuration (session command + theme colors)
- Zodiac brutalist color scheme (configurable)

## Dependencies

- [greetd](https://sr.ht/~kennylevinsen/greetd/) (the login daemon)
- Rust toolchain (to build)

## Install

```sh
cargo build --release
./install.sh
```

This installs the binary to `/usr/local/bin/grxxt` and config files to `/etc/greetd/`. An existing greetd config is backed up automatically.

Then enable greetd:

```sh
sudo systemctl enable greetd
```

## Configuration

`/etc/greetd/grxxt.toml`:

```toml
session = "/usr/local/bin/start-hyprland.sh"

[theme]
background = "#0b0a13"
foreground = "#f6f1e3"
accent = "#f1c35f"
error = "#d14b64"
```

All fields are optional and fall back to the defaults shown above.

## Key Bindings

| Key | Action |
|-----|--------|
| Tab / Shift+Tab | Switch fields |
| Enter | Next field / submit |
| F1 | Shutdown |
| F2 | Reboot |
| F3 | Suspend |
| Esc | Quit (dev only) |

## License

MIT
