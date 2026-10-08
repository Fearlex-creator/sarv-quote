# Exact final5 observer copies

These scripts read/compile the repository's `scripts/product-qa.py`, substituting an immutable binary from `SARV_QUOTE_DIAG_BINARY`. They add external observations only; production Rust files are untouched. `wrapper-sha256.json` hashes these exact scripts and the parent summary also pins `product-qa.py`.

Use the final5 environment, hash and commands in [the parent README](../README.md). `full28.py` starts from the protected synthetic quotation-smoke source; `disclosure4.py`, `modal3.py`, `failure3.py` and `escape6.py` use a fresh clone of the printed full28 profile with `--after-approval`. The final `escape6.py` matches the successful corrected run. Original search-focused and row-toggle failures remain in separate raw evidence; they used earlier observer versions and are not relabeled as final passes.
