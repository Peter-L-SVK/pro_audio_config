#!/bin/bash

# Pro Audio Config Uninstaller
set -e

echo "=== Uninstalling Pro Audio Config ==="

# Remove binary
echo "Removing binary..."
sudo rm -f /usr/local/bin/pro-audio-config

# Remove icons (all sizes that might have been installed)
echo "Removing icons..."
sudo rm -f /usr/local/share/icons/hicolor/16x16/apps/pro-audio-config.png
sudo rm -f /usr/local/share/icons/hicolor/22x22/apps/pro-audio-config.png
sudo rm -f /usr/local/share/icons/hicolor/24x24/apps/pro-audio-config.png
sudo rm -f /usr/local/share/icons/hicolor/32x32/apps/pro-audio-config.png
sudo rm -f /usr/local/share/icons/hicolor/48x48/apps/pro-audio-config.png
sudo rm -f /usr/local/share/icons/hicolor/64x64/apps/pro-audio-config.png
sudo rm -f /usr/local/share/icons/hicolor/128x128/apps/pro-audio-config.png
sudo rm -f /usr/local/share/icons/hicolor/256x256/apps/pro-audio-config.png
sudo rm -f /usr/local/share/icons/hicolor/512x512/apps/pro-audio-config.png

# Remove desktop entry
echo "Removing desktop entry..."
sudo rm -f /usr/local/share/applications/pro-audio-config.desktop

# Remove application data directory
echo "Removing application data..."
sudo rm -rf /usr/local/share/pro-audio-config/

# Remove empty icon directories (optional cleanup)
echo "Cleaning up empty directories..."
sudo rmdir /usr/local/share/icons/hicolor/16x16/apps/ 2>/dev/null || true
sudo rmdir /usr/local/share/icons/hicolor/22x22/apps/ 2>/dev/null || true
sudo rmdir /usr/local/share/icons/hicolor/24x24/apps/ 2>/dev/null || true
sudo rmdir /usr/local/share/icons/hicolor/32x32/apps/ 2>/dev/null || true
sudo rmdir /usr/local/share/icons/hicolor/48x48/apps/ 2>/dev/null || true
sudo rmdir /usr/local/share/icons/hicolor/64x64/apps/ 2>/dev/null || true
sudo rmdir /usr/local/share/icons/hicolor/128x128/apps/ 2>/dev/null || true
sudo rmdir /usr/local/share/icons/hicolor/256x256/apps/ 2>/dev/null || true
sudo rmdir /usr/local/share/icons/hicolor/512x512/apps/ 2>/dev/null || true
sudo rmdir /usr/local/share/icons/hicolor/16x16/ 2>/dev/null || true
sudo rmdir /usr/local/share/icons/hicolor/22x22/ 2>/dev/null || true
sudo rmdir /usr/local/share/icons/hicolor/24x24/ 2>/dev/null || true
sudo rmdir /usr/local/share/icons/hicolor/32x32/ 2>/dev/null || true
sudo rmdir /usr/local/share/icons/hicolor/48x48/ 2>/dev/null || true
sudo rmdir /usr/local/share/icons/hicolor/64x64/ 2>/dev/null || true
sudo rmdir /usr/local/share/icons/hicolor/128x128/ 2>/dev/null || true
sudo rmdir /usr/local/share/icons/hicolor/256x256/ 2>/dev/null || true
sudo rmdir /usr/local/share/icons/hicolor/512x512/ 2>/dev/null || true
sudo rmdir /usr/local/share/icons/hicolor/ 2>/dev/null || true

# Update icon cache
echo "Updating icon cache..."
sudo gtk-update-icon-cache -f /usr/local/share/icons/hicolor/ || true

echo "=== Uninstall Complete ==="
echo "Pro Audio Config has been completely removed from your system."
