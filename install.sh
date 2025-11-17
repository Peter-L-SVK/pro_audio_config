#!/bin/bash

# Pro Audio Config Installer
set -e

echo "=== Installing Pro Audio Config ==="

# Build the release binary
echo "Building the application..."
cargo build --release

# Create directories if they don't exist
echo "Creating installation directories..."
sudo mkdir -p /usr/local/bin/
sudo mkdir -p /usr/local/share/pro-audio-config/

# Create ALL icon directories
echo "Creating icon directories..."
sudo mkdir -p /usr/local/share/icons/hicolor/16x16/apps/
sudo mkdir -p /usr/local/share/icons/hicolor/22x22/apps/
sudo mkdir -p /usr/local/share/icons/hicolor/24x24/apps/
sudo mkdir -p /usr/local/share/icons/hicolor/32x32/apps/
sudo mkdir -p /usr/local/share/icons/hicolor/48x48/apps/
sudo mkdir -p /usr/local/share/icons/hicolor/64x64/apps/
sudo mkdir -p /usr/local/share/icons/hicolor/128x128/apps/
sudo mkdir -p /usr/local/share/icons/hicolor/256x256/apps/
sudo mkdir -p /usr/local/share/icons/hicolor/512x512/apps/

sudo mkdir -p /usr/local/share/applications/

# Copy the binary
echo "Installing binary..."
sudo cp target/release/pro-audio-config /usr/local/bin/pro-audio-config

# Copy the icons (only if they exist)
echo "Installing icons..."
if [ -f "./icons/16x16/icon.png" ]; then
    sudo cp ./icons/16x16/icon.png /usr/local/share/icons/hicolor/16x16/apps/pro-audio-config.png
fi
if [ -f "./icons/32x32/icon.png" ]; then
    sudo cp ./icons/32x32/icon.png /usr/local/share/icons/hicolor/32x32/apps/pro-audio-config.png
fi
if [ -f "./icons/48x48/icon.png" ]; then
    sudo cp ./icons/48x48/icon.png /usr/local/share/icons/hicolor/48x48/apps/pro-audio-config.png
fi
if [ -f "./icons/256x256/icon.png" ]; then
    sudo cp ./icons/256x256/icon.png /usr/local/share/icons/hicolor/256x256/apps/pro-audio-config.png
fi

# Create desktop entry
echo "Creating desktop entry..."
sudo tee /usr/local/share/applications/pro-audio-config.desktop > /dev/null <<EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=Pro Audio Config
Comment=Configure professional audio settings
Exec=/usr/local/bin/pro-audio-config
Icon=pro-audio-config
Categories=Audio;Settings;
Terminal=false
StartupNotify=true
EOF

# Update icon cache (optional)
echo "Updating icon cache..."
sudo gtk-update-icon-cache -f /usr/local/share/icons/hicolor/ || true

# Set permissions
echo "Setting permissions..."
sudo chmod 755 /usr/local/bin/pro-audio-config
sudo chmod 644 /usr/local/share/icons/hicolor/*/apps/pro-audio-config.png 2>/dev/null || true
sudo chmod 644 /usr/local/share/applications/pro-audio-config.desktop

echo ""
echo "=== Installation Complete ==="
echo "Pro Audio Config has been installed to:"
echo "  Binary: /usr/local/bin/pro-audio-config"
echo "  Icons: /usr/local/share/icons/hicolor/*/apps/pro-audio-config.png"
echo "  Desktop entry: /usr/local/share/applications/pro-audio-config.desktop"
echo ""
echo "You can now run 'pro-audio-config' from terminal or find it in your application menu."
