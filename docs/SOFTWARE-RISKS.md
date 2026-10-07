# Software risk follow-up

Publication snapshot checks: Rust formatting, Clippy with warnings denied, **84 ordinary tests passed**, release build. The ordinary run skips one graphical test; it passed separately on the preceding v1 build. No CI workflow has been configured.

| Risk | Outcome and evidence |
| --- | --- |
| Bounded large catalog/order calculation | Passed synthetic release probes at 1,000, 10,000 and 50,000 products, with 100/1,000-line quotes. All phases bounded to 60 seconds and process RSS to 512 MiB. Original 50,000-product/1,000-line preview took 9.08 seconds; the repeated normalization defect was fixed with a disposable index, without changing stored schemas or matching precedence. At 10,000 products, indexed 1,000-line preview measured 16.16 ms versus 1,474.38 ms before the fix on this machine. These are individual local measurements, not service-level guarantees. |
| Large native catalog import preview | Fixed unbounded widget creation: at most 100 source products render per page; all products remain reachable. Two pagination tests cover up to 50,000 products. Actual large native pagination/responsiveness verification remains pending. |
| Long document rendering/UI | A 1,000-line synthetic PDF took about 8–12 seconds under varying concurrent load. Preview/PDF/CSV work now runs on an immutable snapshot in a background worker; duplicate requests are guarded. Worker cleanup tests pass. Actual large native responsiveness verification remains pending. |
| Interrupted committed/uncommitted writes | Passed seven owned-child process scenarios plus worker harness test, using actual SIGKILL only on spawned children. Store candidate/commit/restore checks and raw SQLite uncommitted WAL recovery preserve validated state. Raw SQLite phase tests are distinguished from Store API tests. No physical power loss was simulated. |
| Interrupted export publication | Passed: requested destinations remain absent or complete and existing files are never overwritten. An interrupted process can leave a hidden .sarv-write-*.tmp staging file; automatic orphan cleanup is not implemented. |
| Accessibility labels | Installed AT-SPI inspection found nameless quantity/inline-entry/discount controls; contextual Persian labels were added. Basic labels were verified in an isolated synthetic profile. Contextual delete-button naming and a complete post-change audit remain pending. This is not a screen-reader usability claim. |
| Screen reader/physical printer | Unavailable for acceptance here: real user/equipment validation remains required. |
| Physical power-loss durability | Unavailable: deliberately not tested on the user's machine. Child-process interruption only establishes logical recovery coverage. |

Large synthetic data, databases, backups, raw machine logs and locally installed launchers remain excluded from GitHub and preserved locally. The repository contains application source, portable test/probe code and selected synthetic image/PDF/CSV documentation assets. The existing normal application window was not closed or modified by this follow-up.
