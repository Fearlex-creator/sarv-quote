# Excel

Local workbooks are the working feature. Microsoft 365 is optional and is not connected in this build.

## Workbook

Schema version `sarv-excel-1`. The data sheet is named `Data`. Amount columns are whole rials, never tomans. SKU, barcode, phone, and national id are text so a leading zero survives. Excel date serials are rejected; type dates as `YYYY-MM-DD` text. A cell that starts with `=`, `+`, `-`, or `@` is prefixed with an apostrophe on export so a spreadsheet does not treat Sarv's text as a formula.

Kinds:

| Kind | Columns |
| --- | --- |
| products | sku, name, unit, barcode, category, sale_price_rial, purchase_price_rial, minimum_stock |
| customers / suppliers | name plus the contact columns written in the template |
| opening_stock | sku, name, unit, warehouse, quantity, unit_cost_rial |
| export only | trial_balance, sales_register |

## Commit

Preview does not write. Commit requires `confirm: true`. One error rejects the whole workbook. The same `request_key` and the same file hash replay the first result. Policy `skip` leaves an existing SKU or party. Policy `update` refreshes prices and contact details and does not rewrite posted invoices.

Opening stock creates a purchase order and a full goods receipt in the same save. Inventory (1301) is debited and GRNI (2201) is credited. It does not post owner equity and it does not create a supplier bill. The preview text says that.

## Limits

Compressed size 1 MB, uncompressed 8 MB, 64 zip entries, 5000 data rows. Paths that escape the zip, absolute names, and `vbaProject` are rejected. Values are the cached cell values. Formulas are not calculated and macros are not opened.

## Microsoft 365

The connector runs only when this computer has `SARV_GRAPH_CLIENT_ID`. Optional names are `SARV_GRAPH_TENANT` and `SARV_GRAPH_REDIRECT_URI`. Sarv does not register the application.

The requested scope is `Files.SelectedOperations.Selected` plus `offline_access`. Microsoft’s OneDrive permission reference says `Files.ReadWrite.Selected` is only for Office file handlers and should not be used to call Microsoft Graph. The current per-file scope still does not grant a workbook because someone typed its item id. An administrator must consent, and OneDrive or SharePoint must permission this app on that file. A 403 means that grant is missing. A client id alone is not a connection. Sarv does not fall back to `Files.ReadWrite` or `Files.ReadWrite.All`.

Connect creates a PKCE challenge and a state value. The owner pastes the redirected address. The refresh token is stored in `.graph-grant.json` with mode 0600. Status JSON never includes the token. Disconnect deletes the file and attempts revocation. The content call is only `GET` or `PUT /me/drive/items/{id}/content` for one id. Redirects drop the bearer token, refuse private and link-local targets, and allow `http` only for `127.0.0.1`. Sarv does not list the drive. Preview and commit use the same row checks as a local file. Push replaces that item only, and only after confirmation. `excel_graph_sync` does not run a background sync.

`live_verified` stays false in status. Contract tests talk to a local mock, not to Microsoft. Without the client id, status is `not_configured` and local `.xlsx` still works. A real tenant has not been activated.

Opening stock is a goods receipt: inventory debit, GRNI credit. It is not owner equity and it is not a supplier bill. Clear GRNI with a bill or a reviewed journal.

Zip limits are the whole compressed file (1 MB), each part's compressed size, the sum of declared uncompressed sizes (8 MB), and the bytes actually inflated (8 MB). Paths, macros, embeddings, and external-link parts are refused.

The libraries are calamine 0.38, rust_xlsxwriter 0.102, zip 9, and ureq 2.12. The historic npm `xlsx` package is not used.
