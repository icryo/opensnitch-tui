#!/bin/bash
# Setup OpenSnitch daemon to connect to TUI

DAEMON_CONFIG="/etc/opensnitchd/default-config.json"
TUI_SOCKET="unix:///tmp/osui.sock"

echo "OpenSnitch TUI Daemon Setup"
echo "============================"

# Check if daemon config exists
if [ ! -f "$DAEMON_CONFIG" ]; then
    echo "ERROR: Daemon config not found at $DAEMON_CONFIG"
    echo "Is OpenSnitch daemon installed?"
    exit 1
fi

# Backup existing config
echo "Backing up current config..."
sudo cp "$DAEMON_CONFIG" "${DAEMON_CONFIG}.backup.$(date +%Y%m%d_%H%M%S)"

# Check current Server.Address
CURRENT_ADDR=$(sudo grep -o '"Address"[[:space:]]*:[[:space:]]*"[^"]*"' "$DAEMON_CONFIG" | head -1 | cut -d'"' -f4)
echo "Current Server.Address: $CURRENT_ADDR"

# Update the config to use TUI socket
echo "Updating Server.Address to: $TUI_SOCKET"

# Use jq if available, otherwise use sed
if command -v jq &> /dev/null; then
    sudo jq '.Server.Address = "unix:///tmp/osui.sock"' "$DAEMON_CONFIG" > /tmp/opensnitchd-config.json
    sudo mv /tmp/opensnitchd-config.json "$DAEMON_CONFIG"
else
    # Fallback to sed (less reliable but works for simple cases)
    sudo sed -i 's|"Address"[[:space:]]*:[[:space:]]*"[^"]*"|"Address": "unix:///tmp/osui.sock"|' "$DAEMON_CONFIG"
fi

echo ""
echo "Configuration updated!"
echo ""
echo "Next steps:"
echo "  1. Start the TUI first:  ./target/release/opensnitch-tui"
echo "  2. Restart the daemon:   sudo systemctl restart opensnitchd"
echo ""
echo "To revert to original config:"
echo "  sudo cp ${DAEMON_CONFIG}.backup.* $DAEMON_CONFIG"
echo "  sudo systemctl restart opensnitchd"
