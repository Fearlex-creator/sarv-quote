#!/usr/bin/env bash
# Cloud QA workaround only. A normal Linux installation needs no mount overlay.
set -euo pipefail
repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
profile_dir="$(realpath "${1:?Pass an isolated QA profile below artifacts}")"
case "$profile_dir/" in "$repo_dir/artifacts/"*) ;; *) echo 'Use an isolated artifacts profile' >&2; exit 2;; esac
source /workspace/sarv-environment/activate.sh
env_dir=/workspace/sarv-environment
export LD_LIBRARY_PATH="$env_dir/tauri-sysroot/usr/lib/x86_64-linux-gnu:$LD_LIBRARY_PATH"
export SARV_DATA_DIR="$profile_dir"
export XDG_CACHE_HOME="$profile_dir/runtime/cache"
export XDG_DATA_HOME="$profile_dir/runtime/data"
export XDG_CONFIG_HOME="$profile_dir/runtime/config"
export XDG_RUNTIME_DIR="$profile_dir/runtime/run"
mkdir -p "$XDG_CACHE_HOME" "$XDG_DATA_HOME" "$XDG_CONFIG_HOME" "$XDG_RUNTIME_DIR"
chmod 700 "$XDG_RUNTIME_DIR"
python - <<'PY'
from pathlib import Path
root = Path('/workspace/sarv-environment/native-runtime')
overlay = root/'overlay-libs'; overlay.mkdir(parents=True,exist_ok=True)
(root/'host-libs').mkdir(exist_ok=True)
for entry in Path('/usr/lib/x86_64-linux-gnu').iterdir():
    target = overlay/entry.name
    if not target.is_symlink() and not target.exists():
        target.symlink_to(root/'host-libs'/entry.name)
helper = overlay/'webkit2gtk-4.1'
if not helper.is_symlink() and not helper.exists():
    helper.symlink_to('/workspace/sarv-environment/tauri-sysroot/usr/lib/x86_64-linux-gnu/webkit2gtk-4.1')
PY
cd "$repo_dir"
exec bwrap --ro-bind / / \
  --ro-bind /usr/lib/x86_64-linux-gnu "$env_dir/native-runtime/host-libs" \
  --ro-bind "$env_dir/native-runtime/overlay-libs" /usr/lib/x86_64-linux-gnu \
  --bind /tmp /tmp --bind "$profile_dir" "$profile_dir" \
  --proc /proc --dev /dev \
  dbus-run-session -- desktop/src-tauri/target/release/sarv-desktop
