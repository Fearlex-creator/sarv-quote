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

`excel_graph_status` returns `configured: false` and `status: not_configured`. The instructions tell the owner what a later registration would need: an application they create, a client id, tenant, and redirect URL, with Files.ReadWrite.Selected and offline_access. This build does not ask for tenant admin consent, does not store a token, and `excel_graph_sync` fails closed. A green "connected" state is not shown.

The libraries are calamine 0.32 and rust_xlsxwriter 0.88. The historic npm `xlsx` package is not used.
