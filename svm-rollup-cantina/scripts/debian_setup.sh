#!/usr/bin/env bash

# Update and upgrade
echo "Updating and upgrading..."
sudo apt update
sudo apt upgrade -y

# Install dependencies
echo "Installing native dependencies..."
sudo apt install -y \
  make \
  clang \
  libssl-dev \
  pkg-config \
  protobuf-compiler \
  build-essential

# Install Rust
echo "Installing Rust..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add cargo to PATH
echo "Adding cargo to PATH..."
source "$HOME"/.cargo/env

# Install risczero
echo "Installing risczero..."
curl -L https://risczero.com/install | bash

# Add risczero to PATH
source "$HOME"/.bashrc
rzup install

echo "Done!"

exit 0
