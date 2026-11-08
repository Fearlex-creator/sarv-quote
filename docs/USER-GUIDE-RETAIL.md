# Retail guide

This page is the shop lock, Excel workbooks, and the till. It is not the full accounting specification. An optional journal dimension is a line tag, not a kol/moein/tafsili chart. Sarv does not submit tax filings.

## Install

Windows, from the `v0.2.0-rc.1` pre-release: run `Sarv-0.2.0-rc.1-windows-x64-setup.exe`. It installs for the current user and adds a Start menu shortcut named Sarv. It does not need an administrator account. Uninstalling removes the program and leaves the shop's books in the user profile. The setup is not code-signed, so Windows may ask for confirmation. Opening the window needs Microsoft Edge WebView2, which current Windows 10 and 11 already have. A portable zip is also published for people who would rather not run a setup program.

Linux: install `Sarv_0.2.0-rc.1_amd64.deb` on Ubuntu or Debian. The package depends on WebKitGTK 4.1 and GTK 3. That package is the installer. It is not a statement that every desktop portal is configured.

The first screen asks for a shop password. There is no default password.

Amounts in Excel and in the engine are rials. If the display unit is toman, the screen labels it as toman (10 rials).

## Lock the shop

Settings → Shop lock. Set the owner name and a password. Write down the recovery code before continuing. It is not shown again and it is not emailed. Add a cashier, accountant, or read-only person from the same page. Sign out from the sidebar when you leave the computer.

## Bring in a workbook

Excel → Download template. Fill the Data sheet. Choose the sheet type, preview, read any row errors, then confirm and commit. A bad row saves nothing. Import products before opening stock. The opening-stock sheet is goods received and not yet invoiced: inventory is debited and GRNI is credited. An opening balance of cash or owner capital is a different entry and is not created by this sheet. It is not a finished supplier bill. Clear GRNI with a supplier bill or a reviewed journal.

Export current rows or the trial balance from the same page. The Microsoft 365 panel stays "not configured" until an owner supplies an application registration. Local files work without that.

## Sell at the till

Point of sale. Open the till and enter the cash you counted in the drawer. Search or scan a barcode and press Enter. Choose cash, bank, on account, or a split. Complete the sale and print the receipt. Cash and bank here mean you recorded the money. Sarv did not charge a card. If the browser is offline, checkout stays disabled. Close the till and enter the counted cash. A difference is kept on the shift for you to review; it is not posted as profit.

## Correct a sale

Use the existing sales return on the invoice. Repeating the same action does not create a second document. Do not delete the invoice.
