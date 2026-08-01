# AGENTS.md

## Project

`grxxt` is a Linux-only Rust TUI greeter for `greetd`. It runs directly on a
TTY, authenticates through the greetd Unix socket, and starts a configured
desktop session. Treat changes as security- and availability-sensitive: a
failure here can expose credentials, leave a TTY unusable, or prevent login.

The project is a single Rust 2021 binary. Keep the implementation small and
prefer the existing synchronous, module-oriented design unless a change has a
clear need for more machinery.

## Repository map

- `src/main.rs`: terminal setup, event loop, key dispatch, and TTY restoration
- `src/app.rs`: form state, validation, and authentication orchestration
- `src/greetd.rs`: greetd IPC client and authentication state machine
- `src/ui.rs`: Ratatui layout and rendering
- `src/avatar.rs`: optional image loading and terminal graphics selection
- `src/config.rs`: TOML loading, defaults, and configuration errors
- `src/theme.rs`: color parsing and runtime theme values
- `src/power.rs`: shutdown, reboot, and suspend commands
- `grxxt.toml`: example application configuration
- `greetd-config.toml`: example greetd configuration
- `install.sh`: system installer that preserves live configuration
- `.github/workflows/ci.yml`: authoritative release-quality checks

## Core invariants

### Authentication

- Never log, print, persist, or include passwords in errors.
- Keep session commands as argument vectors parsed with `shell_words`; do not
  introduce `sh -c`, `bash -c`, or equivalent shell evaluation.
- The supported PAM flow is zero or more informational/error messages followed
  by at most one secret prompt. Reject visible prompts and additional secret
  prompts, canceling the greetd session when possible.
- Do not guess answers from prompt text or silently broaden authentication
  support. Multi-factor or multi-prompt support requires an explicit UI and
  state-machine design.
- Clear the password after failed authentication and keep the user able to try
  again.
- Unit tests must use the greetd client abstraction. Do not require a live PAM
  stack or greetd daemon for the normal test suite.

### Terminal lifecycle

- Raw mode, alternate-screen entry, cursor visibility, and cleanup must remain
  balanced on every normal error path.
- Establish cleanup guards immediately after acquiring terminal state.
- Avoid process termination APIs or panic paths that bypass destructors.
- Do not write ordinary output to stdout while the TUI owns the terminal.

### Configuration and sessions

- A missing config uses documented defaults. An unreadable or malformed config
  must return a clear error rather than silently selecting another session.
- Keep `/etc/greetd/grxxt.toml` as the installed configuration path. If local
  config precedence changes, update tests and README documentation together.
- The default session may be Hyprland, but session parsing and execution must
  remain compositor-agnostic.

### Optional and privileged features

- Avatar failures must never block login. Bound image dimensions and decoded
  memory before allocating the full image, not only after decoding.
- Power commands must not invoke a shell. Reap child processes and make command
  or policy failures observable in application state.
- Tests must never execute live shutdown, reboot, suspend, or session-start
  actions.

## Rust conventions

- Follow the lint policy in `Cargo.toml`. Production code must not use `unwrap`,
  `expect`, `panic!`, `todo!`, `unimplemented!`, or `unsafe` without an explicit,
  reviewed justification.
- Every lint allowance must be narrow and include a `reason`.
- Prefer typed errors at module boundaries and add context where an operator can
  act on it.
- Keep rendering separate from state transitions and external side effects.
- Use saturating arithmetic for terminal geometry and account for Unicode
  display width rather than assuming one cell per Rust `char`.
- Add tests with behavior changes. Authentication changes need state-transition
  and call-order assertions; rendering changes need `TestBackend` coverage,
  including a small terminal.
- Do not make unrelated dependency or formatting churn in focused changes.

## Required checks

Run these before handing off a change:

```sh
cargo fmt --all -- --check
cargo test --locked --all-targets --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo package --locked
```

Also run `cargo build --release --locked` for installer, packaging, dependency,
or release-related changes. Run `bash -n install.sh` after editing the installer.

Clippy lints can change between Rust releases. If local Clippy and GitHub CI
disagree, use the toolchain selected by CI and treat the CI result as
authoritative; do not suppress a new lint without evaluating the suggested
code change.

For dependency work, inspect reverse dependency paths with `cargo tree -i` and
run `cargo audit`. Distinguish exploitable vulnerabilities from informational
or soundness warnings in the handoff.

## Documentation and release consistency

- Keep `README.md`, example TOML files, key bindings, and behavior synchronized.
- Do not claim a crates.io or GitHub release exists until it is published.
- Preserve existing `/etc/greetd` files in installer changes. New examples may
  be installed alongside them, but live configuration must not be overwritten.
- Confirm the packaged file list with `cargo package --list` when adding assets
  or top-level files.
