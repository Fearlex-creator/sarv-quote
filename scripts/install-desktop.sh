#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
data_home="${XDG_DATA_HOME:-$HOME/.local/share}"
if [[ -z "$data_home" ]]; then
  data_home="$HOME/.local/share"
fi
app_dir="$data_home/sarv/bin"
applications_dir="$data_home/applications"
books_dir="$data_home/ir.sarv.accounting"

if [[ "${1:-}" == "--uninstall" ]]; then
  rm -f "$app_dir/sarv-desktop" "$app_dir/sarv-agent" "$applications_dir/ir.sarv.accounting.desktop"
  rm -f "$data_home/icons/hicolor"/*/apps/ir.sarv.accounting.png
  rmdir "$app_dir" 2>/dev/null || true
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$applications_dir" || true
  fi
  echo "Uninstalled Sarv desktop files."
  echo "Accounting books were kept at: $books_dir"
  exit 0
fi

if [[ -f "$script_dir/sarv-desktop" ]]; then
  binary="$script_dir/sarv-desktop"
  agent=""
  icon_src="$script_dir/sarv.png"
  desktop_in="$script_dir/ir.sarv.accounting.desktop.in"
else
  repo_dir="$(cd "$script_dir/.." && pwd)"
  binary="$repo_dir/target/release/sarv-desktop"
  agent="$repo_dir/target/release/sarv-agent"
  icon_src="$repo_dir/assets/brand/sarv.png"
  desktop_in=""
fi

if [[ ! -x "$binary" ]]; then
  echo 'Build first: cargo build --release -p sarv-desktop' >&2
  exit 1
fi

icon_size="$(file -b "$icon_src" 2>/dev/null | sed -n 's/.* \([0-9]\+ x [0-9]\+\).*/\1/p' | tr -d ' ' || true)"
if [[ -z "$icon_size" ]]; then
  icon_size="128x128"
fi
icon_dir="$data_home/icons/hicolor/${icon_size}/apps"

install -d "$app_dir" "$applications_dir" "$icon_dir"
install -m 755 "$binary" "$app_dir/sarv-desktop"
if [[ -n "$agent" && -x "$agent" ]]; then
  install -m 755 "$agent" "$app_dir/sarv-agent"
fi
install -m 644 "$icon_src" "$icon_dir/ir.sarv.accounting.png"

for retired_entry in ir.sarv.quote.demo.desktop ir.sarv.quote.v1.desktop; do
  if [[ -f "$applications_dir/$retired_entry" ]]; then
    install -d "${app_dir%/bin}/retired-launchers"
    mv --backup=numbered "$applications_dir/$retired_entry" "${app_dir%/bin}/retired-launchers/"
  fi
done

if [[ -n "$desktop_in" && -f "$desktop_in" ]]; then
  sed "s|@APP_DIR@|$app_dir|g" "$desktop_in" > "$applications_dir/ir.sarv.accounting.desktop"
else
  cat > "$applications_dir/ir.sarv.accounting.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Sarv
Name[fa]=سرو
Comment=Local accounting
Comment[fa]=حسابداری محلی
Keywords=accounting;invoice;حسابداری;
TryExec=$app_dir/sarv-desktop
Exec="$app_dir/sarv-desktop"
Icon=ir.sarv.accounting
Terminal=false
Categories=Office;Finance;
StartupNotify=true
EOF
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$applications_dir" || true
fi
echo "Installed Sarv in $app_dir. Open Sarv from your application launcher."
