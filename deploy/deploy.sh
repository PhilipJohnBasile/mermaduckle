#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 1 ]; then
  echo "Usage: $0 <host> [user] [remote_dir]"
  echo "Example: $0 203.0.113.10 ubuntu /opt/mermaduckle"
  exit 1
fi

HOST="$1"
USER="${2:-$USER}"
REMOTE_DIR="${3:-/opt/mermaduckle}"
BINARY="target/release/mermaduckle-server"

echo "Building release binary..."
cargo build --release -p mermaduckle-server

echo "Preparing remote directory $REMOTE_DIR on $USER@$HOST..."
ssh "$USER@$HOST" "sudo mkdir -p $REMOTE_DIR && sudo chown $USER:$USER $REMOTE_DIR"

echo "Copying binary to remote host..."
scp "$BINARY" "$USER@$HOST:$REMOTE_DIR/mermaduckle-server"

echo "Copying systemd unit and reloading..."
scp deploy/mermaduckle.service "$USER@$HOST:/tmp/mermaduckle.service"
ssh "$USER@$HOST" "sudo mv /tmp/mermaduckle.service /etc/systemd/system/mermaduckle.service && sudo systemctl daemon-reload && sudo systemctl enable --now mermaduckle"

echo "Deploy complete. Backend running under systemd; proxy with Caddy/Nginx as desired."
