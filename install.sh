#!/bin/bash
# Install grxxt greeter
set -e

echo "Installing grxxt..."

# Install binary
sudo install -Dm755 target/release/grxxt /usr/local/bin/grxxt

# Install examples, preserving any live configuration
sudo install -Dm644 grxxt.toml /etc/greetd/grxxt.toml.example
if [[ -e /etc/greetd/grxxt.toml ]]; then
    echo "Preserved existing /etc/greetd/grxxt.toml"
else
    sudo install -Dm644 grxxt.toml /etc/greetd/grxxt.toml
fi

sudo install -Dm644 greetd-config.toml /etc/greetd/config.toml.grxxt.example
if [[ -e /etc/greetd/config.toml ]]; then
    echo "Preserved existing /etc/greetd/config.toml"
    echo "Compare it with /etc/greetd/config.toml.grxxt.example"
else
    sudo install -Dm644 greetd-config.toml /etc/greetd/config.toml
fi

echo "Done! Enable greetd with: sudo systemctl enable greetd"
echo "Test with: sudo systemctl start greetd (or switch to TTY1)"
