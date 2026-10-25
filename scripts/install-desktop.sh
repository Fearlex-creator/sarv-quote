#!/usr/bin/env bash
set -euo pipefail
repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app_dir="${XDG_DATA_HOME:-$HOME/.local/share}/sarv/bin"
applications_dir="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
icon_dir="${XDG_DATA_HOME:-$HOME/.local/share}/icons/hicolor/128x128/apps"
binary="$repo_dir/desktop/src-tauri/target/release/sarv-desktop"
agent="$repo_dir/target/release/sarv-agent"
if [[ ! -x "$binary" || ! -x "$agent" ]]; then
  echo 'Build first: pnpm desktop:build --no-bundle' >&2
  exit 1
fi
install -d "$app_dir" "$applications_dir" "$icon_dir"
install -m 755 "$binary" "$app_dir/sarv-desktop"
install -m 755 "$agent" "$app_dir/sarv-agent"
install -m 644 "$repo_dir/desktop/src-tauri/icons/128x128.png" "$icon_dir/ir.sarv.accounting.png"
for retired_entry in ir.sarv.quote.demo.desktop ir.sarv.quote.v1.desktop; do
  if [[ -f "$applications_dir/$retired_entry" ]]; then
    install -d "${app_dir%/bin}/retired-launchers"
    mv --backup=numbered "$applications_dir/$retired_entry" "${app_dir%/bin}/retired-launchers/"
  fi
done
cat > "$applications_dir/ir.sarv.accounting.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Sarv
Name[fa]=سرو
Comment=Local accounting
Exec="$app_dir/sarv-desktop"
Icon=ir.sarv.accounting
Terminal=false
Categories=Office;Finance;
StartupNotify=true
EOF
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$applications_dir"
fi
echo "Installed Sarv in $app_dir. Open Sarv from your application launcher."
