#!/usr/bin/env bash
set -euo pipefail

echo "=== 1. Format check ==="
cargo fmt --all -- --check

echo "=== 2. Clippy (workspace, all targets, -D warnings) ==="
cargo clippy --workspace --all-targets -- -D warnings

echo "=== 3. Workspace build ==="
cargo build --workspace --all-targets

echo "=== 4. Engine tests ==="
cargo test -p sarv-quote

echo "=== 5. Engine PDF export tests ==="
cargo test -p sarv-quote --features pdf-export

echo "=== 6. Desktop headless tests ==="
cargo test -p sarv-desktop --bin sarv-desktop

echo "=== 7. Workflow YAML syntax ==="
python3 -c "import yaml,sys; [yaml.safe_load(open(f)) for f in sys.argv[1:]]" .github/workflows/*.yml

echo "=== 8. Structural & Done-When checks ==="
test -f rust-toolchain.toml
test -f .github/workflows/ci.yml
test -f .github/workflows/release.yml
test ! -e crates/sarv-quote/assets
test -f docs/DESIGN.md
test -f docs/RELEASING.md
test -f CHANGELOG.md
test -f docs/adr/0008-engine-owned-numbering.md
test -f docs/adr/0009-year-end-closing.md
test -f docs/adr/0010-vat-on-documents.md
test -f docs/adr/0011-bank-reconciliation.md
test -f docs/adr/0012-rune-desktop.md
test $(wc -l < crates/sarv-desktop/src/main.rs) -lt 140
test -f crates/sarv-desktop/src/tests/harness.rs

echo "=== ALL VERIFY CHECKS PASSED ==="
