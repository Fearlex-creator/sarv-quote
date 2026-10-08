# Final5 completion diagnostics and controlled cancellation

This bundle retains the exact historical freeze4 procurement file, its presentation-only 4→5 patch, two isolated observers, raw reports/logs/key events and fresh final5 screenshots. Current binary is `sarv-M4-freeze5` SHA7b390bb6287b6803081b743e676f196de17840e9e6346e0d4c507f4d3fa81a4a; source aggregate6b1996a41ed86807a91a18820f7fce4bb167e99399403b3fd25cf1b14e4aa36c. The original source snapshot SHA is c610cf1e68bad680a18ce7a781dd06968057e88c694c4de828e758111c5063eb, new procurement source SHA b054cff7b7fbdb20396e6852a9094ecc6e14d7b4cc5b5302d30b776b60abc4cb.

The completion observer passed10 checks: actual source ready and valid/rejected preview status are **uninstrumented**. The rejected preview uses valid Gregorian2028-03-22 outside the synthetic fiscal year, reaching the real background kernel validation. Three same-text local/output/global errors and absent working footer are asserted. Changing back to2026-03-22 produces the real valid PO180 preview, explicitly reviewed/unposted. Full serialized SQLite payload and generation remain exactly equal after every read/review/cancel.

Only source cancellation uses the scheduling shim `worker-delay.c`. It forwards the real pthread_setname_np call, then holds only the same owned worker named sarv-export while the private flag exists, with a hard5000ms timeout. It never delays GTK main thread or changes input/result/financial writes. The observer sets the flag after all startup/refresh work, activates a real REC-B detail, confirms the worker trace and reading status, sends actual Escape, waits for dialog closure, releases the flag and asserts explicit canceled completion/no stale status. Actual held time1770.145ms; timeout0. Source C and compiledSO hashes plus raw monotonic trace are preserved in the report. The SO itself is intentionally excluded from this portable bundle. This case is synthetic scheduling evidence and is excluded from performance measurements.

Compile locally in the declared cloud executor:

```sh
cc -shared -fPIC docs/assets/accounting-m4/native/completion-diagnostics/worker-delay.c -o /tmp/sarv-M4-native-worker-delay.so -ldl -pthread
source /workspace/sarv-environment/activate.sh
G_DEBUG=fatal-criticals SARV_QA_BINARY=/workspace/sarv-evidence/bin/sarv-M4-freeze5 SARV_M4_NATIVE_FIXTURE=/workspace/sarv-quote/artifacts/procurement-native/run-av5d43ov/full SARV_M3_NATIVE_FIXTURE=/workspace/sarv-quote/artifacts/procurement-native/run-av5d43ov/M3-before dbus-run-session -- /usr/bin/python3 docs/assets/accounting-m4/native/completion-diagnostics/completion-observer.py
```

Observers copy synthetic SQLite through read-only source connections into new disposable profiles. No live/customer data is used. The uninstrumented preview stage uses the real native picker/form signals to prepare an unposted12×15 order; external XTest owns review/rejection/cancel afterward.

The separate modal probe passed2 read-only checks: actual Tab focuses the disclosure itself in AT-SPI, visible blue ring is captured, Space expands, Escape closes and full state+generation remain equal. The main-window native capture did not receive the harmless Shift key inside this modal, recorded as false rather than invented geometry. The modal screenshots prove visible focus; numeric modal bounds remain unmeasured. Sixteen settled main-page focus bounds in the separate short report remain valid GTK geometry evidence.

All four actual application logs in this set are empty under fatal-criticals. Runner infrastructure output is retained. The native owner personally viewed all status/focus screenshots. Pure dead/stale callback branches were source-reviewed rather than individually injected by this observer. No accounting validation, posting, schema, retry or policy behavior changed in the completion patch.
