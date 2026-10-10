// Debug-only native IPC integration test, explicitly enabled by an environment
// flag and run against FREE_ACCOUNTING_DATA_DIR in /tmp during verification.
(async () => {
  const invoke = window.__TAURI_INTERNALS__.invoke;
  const read = (action, payload = {}) => invoke('accounting_view', {action, payload});
  const mutate = (action, payload = {}) => invoke('accounting_mutate', {action, payload});
  const workspace = await read('workspace');
  if (workspace.contacts.some(c => c.name === 'Native IPC smoke passed')) return;
  if (workspace.accounts.length) throw new Error('Native smoke requires an empty isolated database');
  const account = await mutate('account', {name:'Native test account',kind:'bank',currency:'USD',openingBalance:'100.00'});
  const now = new Date();
  const date = `${now.getFullYear()}-${String(now.getMonth()+1).padStart(2,'0')}-${String(now.getDate()).padStart(2,'0')}`;
  const transaction = await mutate('transaction', {date,kind:'expense',amount:'0.29',accountId:account.id,payee:'Native decimal test',categoryId:'office'});
  const detail = await read('detail', {id:transaction.id});
  const page = await read('ledger', {currency:'USD',account:account.id});
  const updated = await read('workspace');
  if (detail.lines.length !== 2 || detail.lines[0].creditMinor !== '29' || detail.lines[1].debitMinor !== '29') throw new Error('Native journal did not balance');
  if (page.rows[0].balanceMinor !== '9971' || updated.accounts[0].balanceMinor !== '9971' || updated.dashboard.expenseMinor !== '29') throw new Error('Native views disagree');
  await mutate('contact', {name:'Native IPC smoke passed',customer:true,notes:'Native WebKit IPC, exact expense, balanced journal, ledger running balance, account balance, and dashboard were verified.'});
  document.title = 'Sarv — native smoke passed';
})().catch(error => console.error('Native IPC smoke failed', error));
