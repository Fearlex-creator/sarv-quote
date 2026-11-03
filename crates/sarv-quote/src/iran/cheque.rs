//! Sayad cheque lifecycle as a pure state machine. `apply` returns the next state and
//! the balanced posting lines (by account *role*; the ledger maps roles to accounts).
//! Postings follow spec §3.6.
use anyhow::{Result, bail, ensure};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Received,
    Issued,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    InHand,
    InCollection,
    Cleared,
    Bounced,
    Spent,
    Returned,
    Issued,
    Voided,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Deposit,
    Clear,
    Bounce,
    Spend,
    ReturnToCustomer,
    Void,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    NotesReceivable,
    NotesInCollection,
    NotesPayable,
    Bank,
    AccountsReceivable,
    AccountsPayable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Line {
    pub role: Role,
    pub debit_rial: i64,
    pub credit_rial: i64,
}
fn dr(role: Role, a: i64) -> Line {
    Line { role, debit_rial: a, credit_rial: 0 }
}
fn cr(role: Role, a: i64) -> Line {
    Line { role, debit_rial: 0, credit_rial: a }
}

/// Registering a cheque: received = Dr Notes receivable / Cr AR; issued = Dr AP / Cr Notes payable.
pub fn open(direction: Direction, amount_rial: i64) -> Result<(State, Vec<Line>)> {
    ensure!(amount_rial > 0, "Cheque amount must be positive");
    Ok(match direction {
        Direction::Received => (State::InHand, vec![dr(Role::NotesReceivable, amount_rial), cr(Role::AccountsReceivable, amount_rial)]),
        Direction::Issued => (State::Issued, vec![dr(Role::AccountsPayable, amount_rial), cr(Role::NotesPayable, amount_rial)]),
    })
}

pub fn apply(direction: Direction, state: State, action: Action, amount_rial: i64) -> Result<(State, Vec<Line>)> {
    ensure!(amount_rial > 0, "Cheque amount must be positive");
    use {Action::*, Role::*, State::*};
    use Direction::Received as Rx;
    use Direction::Issued as Tx;
    let a = amount_rial;
    Ok(match (direction, state, action) {
        (Rx, InHand, Deposit) => (InCollection, vec![dr(NotesInCollection, a), cr(NotesReceivable, a)]),
        (Rx, InCollection, Clear) => (Cleared, vec![dr(Bank, a), cr(NotesInCollection, a)]),
        (Rx, InHand, Clear) => (Cleared, vec![dr(Bank, a), cr(NotesReceivable, a)]),
        (Rx, InCollection, Bounce) => (Bounced, vec![dr(AccountsReceivable, a), cr(NotesInCollection, a)]),
        (Rx, InHand, Spend) => (Spent, vec![dr(AccountsPayable, a), cr(NotesReceivable, a)]),
        (Rx, InHand, ReturnToCustomer) => (Returned, vec![dr(AccountsReceivable, a), cr(NotesReceivable, a)]),
        (Tx, Issued, Clear) => (Cleared, vec![dr(NotesPayable, a), cr(Bank, a)]),
        // A bounced or voided issued cheque restores the supplier liability.
        (Tx, Issued, Bounce) => (Bounced, vec![dr(NotesPayable, a), cr(AccountsPayable, a)]),
        (Tx, Issued, Void) => (Voided, vec![dr(NotesPayable, a), cr(AccountsPayable, a)]),
        (d, s, act) => bail!("Cheque ({d:?}) in state {s:?} cannot {act:?}"),
    })
}
