# Point of sale

The till is a screen over the existing sale. Checkout saves a draft, posts the order, fulfills the invoice, and records a payment. Stock, the invoice, and the journal stay on the kernel. The payment reference is `POS:{shift id}:{invoice}`.

Open the till with the counted float. That float is not revenue. Close compares counted cash with cash payments whose reference starts with that shift and whose funds role is cash, including reversals. The difference is stored on the shift. Close does not post a journal, so a variance is not silently booked as income or expense.

Hold keeps the cart JSON on the book. Resume loads it and deletes the hold. A ref lock ignores a second checkout click while the first is in flight. Each post has its own request key, so a dropped reply can be retried without a second invoice when the same key is sent again.

Negative stock is rejected by the engine. The screen says card and bank amounts are manual records. Sarv does not contact a bank or a terminal. An offline browser disables checkout so a sale cannot look saved when it was not.

The receipt uses `body.pos-printing` and an 80 mm print rule. That is CSS, not a tested printer driver. A keyboard wedge scanner is an Enter-terminated barcode in the search field. It was not tested with a physical scanner.

The web manifest lets a browser install the shell. There is no service worker and no offline queue.
