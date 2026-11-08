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

The local agent listens only on loopback. Its token is not a shop password. After a shop lock exists, a token that was enabled earlier, or a grant with no role, is refused until the owner approves it again. The owner can grant readonly, cashier, or accountant. That grant is checked on every agent call, including exports the agent is allowed to request. Revoking the grant deletes the connection file. The agent cannot create users, reset the lock, or start a Microsoft 365 sign-in.

The SQLite file, including `app_security`, is not encrypted at rest. The password stops commands. It does not stop someone who can read the profile directory.

Tauri file exports (`save_report`) open the store and run the same role check before writing a PDF, CSV, or backup.
