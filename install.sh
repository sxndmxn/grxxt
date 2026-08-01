#!/bin/bash
# Install grxxt greeter
set -euo pipefail

echo "Installing grxxt..."

if [[ ! -x target/release/grxxt ]]; then
    echo "Missing target/release/grxxt; run: cargo build --release --locked" >&2
    exit 1
fi

destdir=${DESTDIR:-}
if [[ -n $destdir && $destdir != /* ]]; then
    echo "DESTDIR must be an absolute path" >&2
    exit 1
fi

install_file() {
    if [[ -n $destdir ]]; then
        install "$@"
    else
        sudo install "$@"
    fi
}

binary_path="${destdir}/usr/local/bin/grxxt"
config_dir="${destdir}/etc/greetd"

# Install binary
install_file -Dm755 target/release/grxxt "$binary_path"

# Install examples, preserving any live configuration
install_file -Dm644 grxxt.toml "$config_dir/grxxt.toml.example"
if [[ -e "$config_dir/grxxt.toml" || -L "$config_dir/grxxt.toml" ]]; then
    echo "Preserved existing $config_dir/grxxt.toml"
else
    install_file -Dm644 grxxt.toml "$config_dir/grxxt.toml"
fi

install_file -Dm644 greetd-config.toml "$config_dir/config.toml.grxxt.example"
if [[ -e "$config_dir/config.toml" || -L "$config_dir/config.toml" ]]; then
    echo "Preserved existing $config_dir/config.toml"
    echo "Compare it with $config_dir/config.toml.grxxt.example"
else
    install_file -Dm644 greetd-config.toml "$config_dir/config.toml"
fi

if [[ -n $destdir ]]; then
    echo "Staged grxxt under $destdir"
else
    echo "Done! Enable greetd with: sudo systemctl enable greetd"
    echo "Test with: sudo systemctl start greetd (or switch to TTY1)"
fi
