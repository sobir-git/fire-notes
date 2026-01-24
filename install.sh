#!/bin/bash
set -e

echo "🔥 Installing Fire Notes..."

# 0. Backup notes/state
BACKUP_ROOT="$HOME/.local/share/fire-notes-backups"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
mkdir -p "$BACKUP_ROOT"
if [ -d "$HOME/.local/share/fire-notes" ]; then
  echo "🛟 Backing up installed notes/state..."
  tar -czf "$BACKUP_ROOT/fire-notes_installed_$TIMESTAMP.tar.gz" -C "$HOME/.local/share" fire-notes
fi

# 1. Build release binary
echo "📦 Building release binary..."
cargo build --release

# 2. Stop running fire-notes instances and install binary
echo "🛑 Stopping any running fire-notes instances..."
if pkill -f fire-notes; then
  echo "Stopped fire-notes processes."
  sleep 1
else
  echo "No running fire-notes processes found."
fi

echo "🚀 Installing to ~/.local/bin..."
mkdir -p ~/.local/bin
cp target/release/fire-notes ~/.local/bin/

# 3. Install icon
echo "🎨 Installing icon..."
mkdir -p ~/.local/share/icons
cp icon.png ~/.local/share/icons/fire-notes.png

# 4. Create desktop entry
echo "📝 Creating desktop entry..."
mkdir -p ~/.local/share/applications
cat > ~/.local/share/applications/fire-notes.desktop <<EOF
[Desktop Entry]
Type=Application
Name=Fire Notes
Comment=Blazing fast markdown editor
Exec=$HOME/.local/bin/fire-notes
Terminal=false
Categories=Utility;TextEditor;
Keywords=markdown;editor;notes;
Icon=$HOME/.local/share/icons/fire-notes.png
EOF

echo "✅ Done! You may need to log out and back in specifically if you use Wayland or some DEs."
echo "You can launch it by typing 'fire-notes' in a new terminal or finding it in your app launcher."

# 5. Relaunch fire-notes if it was running
echo "🚀 Relaunching fire-notes..."
if command -v fire-notes >/dev/null 2>&1; then
  fire-notes &
  echo "Fire Notes relaunched in the background."
else
  echo "fire-notes command not found in PATH."
fi
