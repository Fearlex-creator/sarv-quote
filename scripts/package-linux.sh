#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || -z "$1" ]]; then
  echo "Usage: $0 <tag>" >&2
  exit 1
fi

tag="$1"
repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
binary="$repo_dir/target/release/sarv-desktop"

if [[ ! -x "$binary" ]]; then
  echo "Missing release binary: $binary" >&2
  echo "Build first: cargo build --release -p sarv-desktop" >&2
  exit 1
fi

out="Sarv-${tag}-linux-x64"
rm -rf "$out" "${out}.tar.gz"
mkdir -p "$out"

cp "$binary" "$out/sarv-desktop"
chmod 755 "$out/sarv-desktop"
cp "$repo_dir/assets/brand/sarv.png" "$out/sarv.png"
cp "$repo_dir/LICENSE" "$out/LICENSE"
cp "$repo_dir/scripts/install-desktop.sh" "$out/install.sh"
chmod 755 "$out/install.sh"

cat > "$out/uninstall.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
exec "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/install.sh" --uninstall "$@"
EOF
chmod 755 "$out/uninstall.sh"

cat > "$out/ir.sarv.accounting.desktop.in" <<'EOF'
[Desktop Entry]
Type=Application
Name=Sarv
Name[fa]=سرو
Comment=Local accounting
Comment[fa]=حسابداری محلی
Keywords=accounting;invoice;حسابداری;
TryExec=@APP_DIR@/sarv-desktop
Exec="@APP_DIR@/sarv-desktop"
Icon=ir.sarv.accounting
Terminal=false
Categories=Office;Finance;
StartupNotify=true
EOF

ldd_deps="$(ldd "$binary" | grep -E 'cairo|pango|fontconfig|freetype|harfbuzz' || true)"

cat > "$out/README.txt" <<EOF
Sarv ${tag} (Linux x64)
========================

Install for the current user:
  bash install.sh

Uninstall (preserves your accounting books):
  bash uninstall.sh

Runtime shared libraries required (install cairo, pango, fontconfig,
freetype, and harfbuzz from your distribution if not already present):
${ldd_deps}
EOF

tar -czf "${out}.tar.gz" "$out"
echo "Created ${out}.tar.gz"
