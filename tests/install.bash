#!/bin/bash
set -euo pipefail

if [[ ! -x target/release/grxxt ]]; then
    echo "Missing target/release/grxxt; run: cargo build --release --locked" >&2
    exit 1
fi

stage_dir=$(mktemp -d)

cleanup() {
    for staged_file in \
        "$stage_dir/usr/local/bin/grxxt" \
        "$stage_dir/etc/greetd/grxxt.toml.example" \
        "$stage_dir/etc/greetd/grxxt.toml" \
        "$stage_dir/etc/greetd/config.toml.grxxt.example" \
        "$stage_dir/etc/greetd/config.toml"; do
        if [[ -e $staged_file || -L $staged_file ]]; then
            unlink "$staged_file"
        fi
    done

    for staged_dir in \
        "$stage_dir/usr/local/bin" \
        "$stage_dir/usr/local" \
        "$stage_dir/usr" \
        "$stage_dir/etc/greetd" \
        "$stage_dir/etc" \
        "$stage_dir"; do
        rmdir "$staged_dir" 2>/dev/null || true
    done
}
trap cleanup EXIT

mkdir -p "$stage_dir/etc/greetd"
ln -s /preserved/grxxt.toml "$stage_dir/etc/greetd/grxxt.toml"
touch "$stage_dir/etc/greetd/config.toml"

DESTDIR="$stage_dir" ./install.sh

cmp target/release/grxxt "$stage_dir/usr/local/bin/grxxt"
cmp grxxt.toml "$stage_dir/etc/greetd/grxxt.toml.example"
cmp greetd-config.toml "$stage_dir/etc/greetd/config.toml.grxxt.example"
test "$(stat -c %a "$stage_dir/usr/local/bin/grxxt")" = 755
test "$(stat -c %a "$stage_dir/etc/greetd/grxxt.toml.example")" = 644
test "$(readlink "$stage_dir/etc/greetd/grxxt.toml")" = /preserved/grxxt.toml
test ! -s "$stage_dir/etc/greetd/config.toml"

unlink "$stage_dir/etc/greetd/grxxt.toml"
unlink "$stage_dir/etc/greetd/config.toml"
DESTDIR="$stage_dir" ./install.sh

cmp grxxt.toml "$stage_dir/etc/greetd/grxxt.toml"
cmp greetd-config.toml "$stage_dir/etc/greetd/config.toml"
test "$(stat -c %a "$stage_dir/etc/greetd/grxxt.toml")" = 644
test "$(stat -c %a "$stage_dir/etc/greetd/config.toml")" = 644

if DESTDIR=relative ./install.sh >/dev/null 2>&1; then
    echo "relative DESTDIR unexpectedly succeeded" >&2
    exit 1
fi

echo "Installer staging checks passed"
