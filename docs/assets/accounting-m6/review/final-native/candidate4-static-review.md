# Independent post-C3 source review

Reviewer: `m6_final_native_review`. Read-only inspection against archived candidate3 source; no Cargo, GUI or network execution. This is a source review of the evolving candidate4 preparation, not new native or release acceptance. Original candidate3 failed evidence and C3-R01 remain unchanged in `review.md` and `result.json`.

## Export repair

The targeted physical-return/supplier-credit repair uses `source_code` to mark exact identity/reference/SKU spans and `localized_label` to preserve those spans while formatting remaining date/quantity/amount/sequence text. Emitted protected spans become LTR isolates. This correctly avoids the rejected alternative of preserving every isolate, which would also leave quantities and dates unlocalized. Validated user strings reject U+2066–U+2069, so allowed user content cannot impersonate the internal FSI/PDI marker. Ordinary historical v1 constructors remain unchanged.

The new regression was read, not executed by this reviewer. It checks both physical and credit previews with mixed IDs, numeric-only paper references, journal identity, Persian45/date/3.000 and unchanged complete State. Actual PDF/CSV native execution remains required.

Two residual paths of C3-R01 were reported promptly before the next native run: reconciliation Return `source_event` labels still construct physical/credit IDs with `ltr`, and reconciliation Return `source_stock_movement` labels still construct SKU with `ltr`. These labels pass through digit formatting and therefore still change numeric code characters under Persian preference. Requested bounded correction: `source_code` for those Return-only identity/SKU spans while retaining historical non-Return constructors; extend reconciliation-preview coverage with mixed/numeric SKU and source IDs. Root reported candidate4 qualification already underway and plans to retain it as superseded, with no candidate4 native execution, then qualify repaired candidate5. No further material source defect was identified in inspected export branches.

Source inspected at this epoch: `src/procurement_export.rs` SHA256 `bbca6e556bbeb2e4bf4827d13aded86a8667817f6221982bfff14dd36e427286`; `tests/purchase_return_exports.rs` `0877afe6fb088bead8d084a31d4f1a5db88f765b87cceebf721fc654cd2f0396`.

## Collapsible app shell

The shell now uses one persistent `OverlaySplitView` with separate native content/sidebar ToolbarViews. Wide mode starts with the sidebar shown and exposes the header toggle/F9; the narrow breakpoint collapses into an overlay. The sidebar has a close button in adaptive mode. Opening focuses the current destination, closing via Escape focuses the toggle, and destination activation hides only an adaptive overlay. `show_workspace_content` preserves the user's manual wide sidebar choice during ordinary navigation. The small core/accounting/settings callsite changes call that same helper. Active dialogs retain their own shortcuts. New/changed sidebar callbacks use weak split references; no new direct strong split/callback ownership cycle was identified.

No material shell source defect identified. Source cannot prove native animation/focus/layout behavior. Required actual candidate5 observations: wide toggle/F9 hide and reopen with expanded content; narrow overlay over stable content; close-button/Escape focus recovery; current-row focus; resize transitions; preserved scrolling/input and unchanged complete saved payload. High contrast, enlarged text, Wayland and screen-reader speech remain explicit limits until tested.

Source inspected at this epoch: `src/ui.rs` `c41f6090d835e46e8547d176f831025070efda4d90361fefb0d9e386f50eefad`; unchanged candidate3 CSS `717e18a97b7b8b34f0931684ef8ed8d12e0c9376925adfca83a542dc0cd199b7`; `src/ui/core.rs` `ca44d726740d6d31e89b74a702841d95af87a3b6ad612956fb630ac33b88daa1`; `src/ui/accounting.rs` `2d02443e543c82b67d96ed83000e205ddbb49043158ff783a93482be34066a45`; `src/ui/settings.rs` `5a0793749b19913290110e4237e0c2a71d8e28030c0acc252f042bdea70a716a`.

This review withholds closure of C3-R01 until the residual constructors are fixed and the new immutable candidate's actual native exports are inspected. It makes no financial-core, full performance or production gate claim.
