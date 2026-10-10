use sarv_quote::{
    domain::State,
    store::{CommittedChange, Store},
};
use std::sync::{Arc, Barrier};

fn worker(
    directory: std::path::PathBuf,
    generation: i64,
    candidate: State,
) -> anyhow::Result<CommittedChange> {
    Store::commit_detached(directory, Store::prepare_candidate(generation, candidate)?)
}

#[test]
fn detached_save_reserves_owner_and_durably_survives_unacknowledged_restart() {
    let dir = tempfile::tempdir().unwrap();
    let mut owner = Store::open(dir.path()).unwrap();
    let (generation, mut candidate) = owner.begin_detached().unwrap();
    candidate.draft.customer = "ذخیره در کارگر".into();
    assert!(owner.begin_detached().is_err());
    assert!(owner.change(|_| Ok(())).is_err());
    assert!(owner.verify_current().is_err());
    assert!(owner.backup(&dir.path().join("during.json")).is_err());
    assert!(owner.restore(State::default()).is_err());
    let path = dir.path().to_path_buf();
    let receipt = std::thread::spawn(move || worker(path, generation, candidate))
        .join()
        .unwrap()
        .unwrap();
    assert!(owner.state.draft.customer.is_empty());
    assert_eq!(
        Store::open(dir.path()).unwrap().state.draft.customer,
        "ذخیره در کارگر"
    );
    owner.finish_detached(Ok(receipt)).unwrap();
    assert_eq!(owner.local_generation(), generation + 1);
    assert_eq!(owner.state.draft.customer, "ذخیره در کارگر");
    owner.change(|_| Ok(())).unwrap();
    let backup = dir.path().join("after.json");
    owner.backup(&backup).unwrap();
    assert_eq!(
        Store::restore_preview(&backup).unwrap().draft.customer,
        "ذخیره در کارگر"
    );
}

#[test]
fn detached_failures_release_reservation_without_losing_state() {
    let dir = tempfile::tempdir().unwrap();
    let mut owner = Store::open(dir.path()).unwrap();
    let (generation, mut candidate) = owner.begin_detached().unwrap();
    candidate.schema = 99;
    let result = worker(dir.path().to_path_buf(), generation, candidate);
    assert!(owner.finish_detached(result).is_err());
    assert_eq!(owner.local_generation(), generation);
    owner
        .change(|s| {
            s.draft.customer = "پس از خطا".into();
            Ok(())
        })
        .unwrap();
    assert_eq!(
        Store::open(dir.path()).unwrap().state.draft.customer,
        "پس از خطا"
    );
}

#[test]
fn concurrent_detached_saves_have_one_winner_and_keep_stale_owner_input() {
    let dir = tempfile::tempdir().unwrap();
    let mut owner = Store::open(dir.path()).unwrap();
    let (generation, initial) = owner.begin_detached().unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let handles = ["اول", "دوم"].map(|name| {
        let barrier = barrier.clone();
        let path = dir.path().to_path_buf();
        let mut candidate = initial.clone();
        candidate.draft.customer = name.into();
        std::thread::spawn(move || {
            barrier.wait();
            (name, worker(path, generation, candidate))
        })
    });
    let mut receipts = handles.map(|h| h.join().unwrap());
    assert_eq!(receipts.iter().filter(|(_, r)| r.is_ok()).count(), 1);
    let winner = receipts.iter_mut().find(|(_, r)| r.is_ok()).unwrap();
    assert_eq!(
        Store::open(dir.path()).unwrap().state.draft.customer,
        winner.0
    );
    let receipt = std::mem::replace(&mut winner.1, Err(anyhow::anyhow!("taken"))).unwrap();
    owner.finish_detached(Ok(receipt)).unwrap();
    let mut stale = Store::open(dir.path()).unwrap();
    let (old_generation, mut pending) = stale.begin_detached().unwrap();
    pending.draft.customer = "قدیمی".into();
    owner
        .change(|s| {
            s.draft.customer = "جدید".into();
            Ok(())
        })
        .unwrap();
    assert!(
        stale
            .finish_detached(worker(dir.path().to_path_buf(), old_generation, pending))
            .is_err()
    );
    assert_eq!(
        Store::open(dir.path()).unwrap().state.draft.customer,
        "جدید"
    );
}

#[test]
fn detached_save_never_recreates_a_missing_database_or_adopts_another_profile() {
    let dir = tempfile::tempdir().unwrap();
    assert!(worker(dir.path().to_path_buf(), 0, State::default()).is_err());
    assert!(!dir.path().join("sarv.sqlite3").exists());
    let mut first = Store::open(dir.path()).unwrap();
    let other = tempfile::tempdir().unwrap();
    let mut second = Store::open(other.path()).unwrap();
    let (generation, mut candidate) = first.begin_detached().unwrap();
    candidate.draft.customer = "اول".into();
    let receipt = worker(dir.path().to_path_buf(), generation, candidate).unwrap();
    assert!(second.adopt_committed(receipt).is_err());
    assert!(second.state.draft.customer.is_empty());
}
