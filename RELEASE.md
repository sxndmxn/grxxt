# Release checklist

1. Confirm `CHANGELOG.md`, `Cargo.toml`, `README.md`, and both example TOML files describe the same behavior and version.
2. Run the full local gate:

   ```sh
   cargo fmt --all -- --check
   cargo test --locked --all-targets --all-features
   cargo clippy --locked --all-targets --all-features -- -D warnings
   cargo package --locked
   cargo build --release --locked
   bash -n install.sh tests/install.bash
   bash tests/install.bash
   cargo audit --deny warnings
   cargo deny --locked --all-features check bans licenses sources
   ```

3. Review `cargo package --locked --list` and test the packaged crate on a disposable TTY with a non-production greetd/PAM setup.
4. Review third-party license obligations before distributing compiled binaries; `cargo deny` enforces the approved dependency expressions but does not generate notices.
5. Enable GitHub private vulnerability reporting and Dependabot security updates, then update `SECURITY.md` with the working private-report link.
6. Protect `master` with a ruleset requiring both CI jobs, update the repository description to match the compositor-agnostic README, then push the release commit and require both jobs to pass.
7. Run `cargo publish --locked --dry-run`, then publish only after reviewing the exact archive.
8. Verify the crates.io page exists before adding a crates.io badge or `cargo install grxxt` instructions.
9. Create and push the signed `v0.1.0` tag, then create the matching GitHub release without changing the tested source.
