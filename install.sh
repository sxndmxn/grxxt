#!/bin/bash
# Install grxxt greeter
set -e

echo "Installing grxxt..."

# Install binary
sudo install -Dm755 target/release/grxxt /usr/local/bin/grxxt

# Install config
sudo install -Dm644 grxxt.toml /etc/greetd/grxxt.toml

# Backup existing greetd config if present
if [[ -f /etc/greetd/config.toml ]]; then
    sudo cp /etc/greetd/config.toml /etc/greetd/config.toml.bak
    echo "Backed up existing config to /etc/greetd/config.toml.bak"
fi

# Install greetd config
sudo install -Dm644 greetd-config.toml /etc/greetd/config.toml

echo "Done! Enable greetd with: sudo systemctl enable greetd"
echo "Test with: sudo systemctl start greetd (or switch to TTY1)"
