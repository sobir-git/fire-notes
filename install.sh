#!/bin/bash
set -e

echo "🔥 Installing Fire Notes..."

# 1. Build release binary
echo "📦 Building release binary..."
cargo build --release

# 2. Install binary
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
