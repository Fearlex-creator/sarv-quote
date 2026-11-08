# Shop lock

The lock is off until an owner sets a password in Settings → Shop lock. There is no shipped password.

Passwords are Argon2id hashes in the `app_security` table. That table is not inside the JSON financial backup. Restoring a JSON backup into a new folder starts unlocked. Copying `sarv.sqlite3` copies the hashes. Sessions last 12 hours. The browser keeps the session token in `sessionStorage` under `sarv.session` and sends it as `session_token`. The database stores only the SHA-256 of the token. Eight failures lock sign-in for 15 minutes. Logout drops the session.

Roles:

| Role | Writes |
| --- | --- |
| owner, admin | All shop commands |
| accountant | Books, Excel commit, backup export. Not restore, user admin, agent setup, or company setup |
| cashier | Till, drafts, fulfillment, customer receipts, sale returns, customers, items, units |
| read-only | Reads, including PDF and CSV export |

A cashier cannot post a manual journal or commit an Excel workbook. The recovery code is shown once at bootstrap, stored as a hash, and is the only reset. There is no email reset. `auth_reset` does not need a session; the code is the credential. Generated passwords are not written into reports.

The local agent, if the owner enables it, still authenticates with its own token and does not pass through this role check. Treat enabling the agent as granting another full local credential.

Tauri file exports (`save_report`) open the store and run the same role check before writing a PDF, CSV, or backup.
