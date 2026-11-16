//! Local app lock: PBKDF2-HMAC-SHA256 authentication, ChaCha20+HMAC-SHA256 document sealing,
//! and persistent progressive lockout.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const DEFAULT_ITERATIONS: u32 = 600_000;
pub const MIN_PASSWORD_LEN: usize = 4;
pub const MAX_ATTEMPTS_BEFORE_LOCK: u32 = 3;

const LOCK_FILE: &str = "app_lock.json";
const SEALED_EXT: &str = "sarvlock";
const SEAL_MAGIC: &[u8; 8] = b"SARVENC1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockRecord {
    pub version: u32,
    pub salt_hex: String,
    pub hash_hex: String,
    #[serde(default)]
    pub wrapped_dek_hex: String,
    pub iterations: u32,
    pub failed_attempts: u32,
    pub locked_until_unix: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifyOutcome {
    Unlocked([u8; 32]),
    WrongPassword {
        failed_attempts: u32,
        attempts_until_lock: u32,
        lockout_secs: i64,
    },
    LockedOut {
        remaining_secs: i64,
        failed_attempts: u32,
    },
}

pub fn lock_file_path(dir: &Path) -> PathBuf {
    dir.join(LOCK_FILE)
}

pub fn is_enabled(dir: &Path) -> bool {
    lock_file_path(dir).is_file()
}

pub fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Progressive lockout duration (seconds) after `failed_attempts` consecutive failures.
pub fn lockout_duration_secs(failed_attempts: u32) -> i64 {
    match failed_attempts {
        0..=2 => 0,
        3 => 30,
        4 => 60,
        5 => 300,
        6 => 900,
        _ => 1800,
    }
}

pub fn load(dir: &Path) -> Result<Option<LockRecord>, String> {
    let path = lock_file_path(dir);
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("Failed to read lock file: {e}"))?;
    let rec: LockRecord = serde_json::from_str(&raw).map_err(|e| format!("Corrupt lock file: {e}"))?;
    Ok(Some(rec))
}

pub fn remaining_lockout_secs(dir: &Path, now: i64) -> i64 {
    match load(dir) {
        Ok(Some(rec)) if rec.locked_until_unix > now => rec.locked_until_unix - now,
        _ => 0,
    }
}

pub fn failed_attempts(dir: &Path) -> u32 {
    match load(dir) {
        Ok(Some(rec)) => rec.failed_attempts,
        _ => 0,
    }
}

fn derive_auth_and_kek(password: &str, salt: &[u8], iterations: u32) -> ([u8; 32], [u8; 32]) {
    let master = pbkdf2_hmac_sha256(password.as_bytes(), salt, iterations.max(1));
    let auth = hmac_sha256(&master, b"sarv-auth-v2");
    let kek = hmac_sha256(&master, b"sarv-kek-v2");
    (auth, kek)
}

/// Set a new app lock password, generating a fresh random DEK. Returns the active DEK.
pub fn set_password(dir: &Path, password: &str, iterations: u32) -> Result<[u8; 32], String> {
    validate_password(password)?;
    let mut dek = [0u8; 32];
    getrandom::fill(&mut dek).map_err(|e| format!("Random generator failed: {e}"))?;
    save_with_dek(dir, password, &dek, iterations)?;
    Ok(dek)
}

fn save_with_dek(dir: &Path, password: &str, dek: &[u8; 32], iterations: u32) -> Result<(), String> {
    let mut salt = [0u8; 16];
    getrandom::fill(&mut salt).map_err(|e| format!("Random generator failed: {e}"))?;
    let iters = iterations.max(1);
    let (auth, kek) = derive_auth_and_kek(password, &salt, iters);
    let wrapped = seal_bytes(&kek, dek)?;
    let rec = LockRecord {
        version: 2,
        salt_hex: encode_hex(&salt),
        hash_hex: encode_hex(&auth),
        wrapped_dek_hex: encode_hex(&wrapped),
        iterations: iters,
        failed_attempts: 0,
        locked_until_unix: 0,
    };
    save_record(dir, &rec)
}

/// Verify `password` against the stored lock record and persist failure/lockout state.
pub fn verify_and_record(dir: &Path, password: &str, now: i64) -> Result<VerifyOutcome, String> {
    let Some(mut rec) = load(dir)? else {
        return Ok(VerifyOutcome::Unlocked([0u8; 32]));
    };
    if rec.locked_until_unix > now {
        return Ok(VerifyOutcome::LockedOut {
            remaining_secs: rec.locked_until_unix - now,
            failed_attempts: rec.failed_attempts,
        });
    }

    let salt = decode_hex(&rec.salt_hex)?;
    let expected = decode_hex(&rec.hash_hex)?;
    let iters = rec.iterations.max(1);

    let (matched, dek) = if rec.version >= 2 {
        let (auth, kek) = derive_auth_and_kek(password, &salt, iters);
        if constant_time_eq(&auth, &expected) {
            let wrapped = decode_hex(&rec.wrapped_dek_hex)?;
            let plain = unseal_bytes(&kek, &wrapped)?;
            let mut key = [0u8; 32];
            if plain.len() != 32 {
                return Err("Corrupt key material in lock file".to_string());
            }
            key.copy_from_slice(&plain);
            (true, key)
        } else {
            (false, [0u8; 32])
        }
    } else {
        // Legacy v1 compatibility
        let derived = pbkdf2_hmac_sha256(password.as_bytes(), &salt, iters);
        (constant_time_eq(&derived, &expected), [0u8; 32])
    };

    if matched {
        if rec.failed_attempts != 0 || rec.locked_until_unix != 0 {
            rec.failed_attempts = 0;
            rec.locked_until_unix = 0;
            save_record(dir, &rec)?;
        }
        return Ok(VerifyOutcome::Unlocked(dek));
    }

    rec.failed_attempts = rec.failed_attempts.saturating_add(1);
    let lockout_secs = lockout_duration_secs(rec.failed_attempts);
    rec.locked_until_unix = if lockout_secs > 0 { now.saturating_add(lockout_secs) } else { 0 };
    save_record(dir, &rec)?;

    Ok(VerifyOutcome::WrongPassword {
        failed_attempts: rec.failed_attempts,
        attempts_until_lock: MAX_ATTEMPTS_BEFORE_LOCK.saturating_sub(rec.failed_attempts),
        lockout_secs,
    })
}

/// Change password after verifying the current password (respecting lockout). Preserves the DEK.
pub fn change_password(dir: &Path, current: &str, next: &str, now: i64, iterations: u32) -> Result<VerifyOutcome, String> {
    validate_password(next)?;
    match verify_and_record(dir, current, now)? {
        VerifyOutcome::Unlocked(dek) => {
            save_with_dek(dir, next, &dek, iterations)?;
            Ok(VerifyOutcome::Unlocked(dek))
        }
        other => Ok(other),
    }
}

/// Disable app lock after verifying the current password (respecting lockout).
pub fn disable(dir: &Path, current: &str, now: i64) -> Result<VerifyOutcome, String> {
    match verify_and_record(dir, current, now)? {
        VerifyOutcome::Unlocked(dek) => {
            unseal_workspace_data(dir, &dek)?;
            let path = lock_file_path(dir);
            if path.exists() {
                fs::remove_file(&path).map_err(|e| format!("Failed to remove lock file: {e}"))?;
            }
            Ok(VerifyOutcome::Unlocked(dek))
        }
        other => Ok(other),
    }
}

pub fn validate_password(password: &str) -> Result<(), String> {
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err("Password must be at least 4 characters.".to_string());
    }
    Ok(())
}

/// Encrypt `sarv.sqlite3`, backup JSON files, and `exports/` documents in `dir` with `dek`.
pub fn seal_workspace_data(dir: &Path, dek: &[u8; 32]) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    let db = dir.join("sarv.sqlite3");
    if db.is_file() {
        seal_file(&db, dek)?;
    }
    for entry in fs::read_dir(dir).map_err(|e| format!("Failed to read books folder: {e}"))? {
        let entry = entry.map_err(|e| format!("Failed to inspect books entry: {e}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == LOCK_FILE || name.starts_with('.') || name.ends_with(SEALED_EXT) {
            continue;
        }
        if name.ends_with(".json") {
            seal_file(&path, dek)?;
        }
    }
    let exports = dir.join("exports");
    if exports.is_dir() {
        for entry in fs::read_dir(&exports).map_err(|e| format!("Failed to read exports folder: {e}"))? {
            let entry = entry.map_err(|e| format!("Failed to inspect export entry: {e}"))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') || name.ends_with(SEALED_EXT) {
                continue;
            }
            seal_file(&path, dek)?;
        }
    }
    Ok(())
}

/// Decrypt all `.sarvlock` files in `dir` and `dir/exports` with `dek`.
pub fn unseal_workspace_data(dir: &Path, dek: &[u8; 32]) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    unseal_dir(dir, dek)?;
    let exports = dir.join("exports");
    if exports.is_dir() {
        unseal_dir(&exports, dek)?;
    }
    Ok(())
}

fn unseal_dir(dir: &Path, dek: &[u8; 32]) -> Result<(), String> {
    let suffix = format!(".{SEALED_EXT}");
    for entry in fs::read_dir(dir).map_err(|e| format!("Failed to read folder: {e}"))? {
        let entry = entry.map_err(|e| format!("Failed to inspect entry: {e}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with(&suffix) {
            continue;
        }
        let orig_name = &name[..name.len() - suffix.len()];
        let target = dir.join(orig_name);
        let sealed = fs::read(&path).map_err(|e| format!("Failed to read sealed file: {e}"))?;
        let plain = unseal_bytes(dek, &sealed)?;
        write_atomic_private(&target, &plain)?;
        fs::remove_file(&path).map_err(|e| format!("Failed to remove sealed file: {e}"))?;
    }
    Ok(())
}

fn seal_file(path: &Path, dek: &[u8; 32]) -> Result<(), String> {
    let plain = fs::read(path).map_err(|e| format!("Failed to read data file: {e}"))?;
    let sealed = seal_bytes(dek, &plain)?;
    let mut sealed_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("data").to_string();
    sealed_name.push('.');
    sealed_name.push_str(SEALED_EXT);
    let sealed_path = path.with_file_name(sealed_name);
    write_atomic_private(&sealed_path, &sealed)?;
    fs::remove_file(path).map_err(|e| format!("Failed to remove unsealed file: {e}"))?;
    Ok(())
}

fn write_atomic_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let tmp = path.with_file_name(format!(".{name}.tmp"));
    fs::write(&tmp, bytes).map_err(|e| format!("Failed to write file: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600));
    }
    fs::rename(&tmp, path).map_err(|e| format!("Failed to save file: {e}"))?;
    Ok(())
}

fn save_record(dir: &Path, rec: &LockRecord) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("Failed to create security directory: {e}"))?;
    let json = serde_json::to_string_pretty(rec).map_err(|e| format!("Failed to serialize lock record: {e}"))?;
    write_atomic_private(&lock_file_path(dir), json.as_bytes())
}

/// Encrypt-then-MAC (ChaCha20 + HMAC-SHA256).
pub fn seal_bytes(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let mut nonce = [0u8; 12];
    getrandom::fill(&mut nonce).map_err(|e| format!("Random generator failed: {e}"))?;
    let enc_key = hmac_sha256(key, b"sarv-enc-v1");
    let mac_key = hmac_sha256(key, b"sarv-mac-v1");

    let mut out = Vec::with_capacity(SEAL_MAGIC.len() + nonce.len() + plaintext.len() + 32);
    out.extend_from_slice(SEAL_MAGIC);
    out.extend_from_slice(&nonce);
    let ct_start = out.len();
    out.extend_from_slice(plaintext);
    chacha20_xor(&enc_key, &nonce, &mut out[ct_start..]);
    let tag = hmac_sha256(&mac_key, &out);
    out.extend_from_slice(&tag);
    Ok(out)
}

pub fn unseal_bytes(key: &[u8; 32], sealed: &[u8]) -> Result<Vec<u8>, String> {
    let min_len = SEAL_MAGIC.len() + 12 + 32;
    if sealed.len() < min_len || &sealed[..SEAL_MAGIC.len()] != SEAL_MAGIC {
        return Err("Invalid or corrupt encrypted data".to_string());
    }
    let body_end = sealed.len() - 32;
    let body = &sealed[..body_end];
    let tag = &sealed[body_end..];
    let mac_key = hmac_sha256(key, b"sarv-mac-v1");
    let expected_tag = hmac_sha256(&mac_key, body);
    if !constant_time_eq(tag, &expected_tag) {
        return Err("Encrypted data authentication failed".to_string());
    }
    let nonce: [u8; 12] = sealed[SEAL_MAGIC.len()..SEAL_MAGIC.len() + 12]
        .try_into()
        .map_err(|_| "Invalid nonce".to_string())?;
    let enc_key = hmac_sha256(key, b"sarv-enc-v1");
    let mut out = sealed[SEAL_MAGIC.len() + 12..body_end].to_vec();
    chacha20_xor(&enc_key, &nonce, &mut out);
    Ok(out)
}

/// RFC 8439 ChaCha20 stream cipher XOR.
fn chacha20_xor(key: &[u8; 32], nonce: &[u8; 12], data: &mut [u8]) {
    let mut state = [0u32; 16];
    state[0] = 0x6170_7865;
    state[1] = 0x3320_646e;
    state[2] = 0x7962_2d32;
    state[3] = 0x6b20_6574;
    for i in 0..8 {
        state[4 + i] = u32::from_le_bytes(key[i * 4..i * 4 + 4].try_into().unwrap());
    }
    state[12] = 1;
    for i in 0..3 {
        state[13 + i] = u32::from_le_bytes(nonce[i * 4..i * 4 + 4].try_into().unwrap());
    }

    for chunk in data.chunks_mut(64) {
        let block = chacha20_block(state);
        for (b, k) in chunk.iter_mut().zip(block.iter()) {
            *b ^= *k;
        }
        state[12] = state[12].wrapping_add(1);
    }
}

fn chacha20_block(initial: [u32; 16]) -> [u8; 64] {
    let mut x = initial;
    for _ in 0..10 {
        quarter_round(&mut x, 0, 4, 8, 12);
        quarter_round(&mut x, 1, 5, 9, 13);
        quarter_round(&mut x, 2, 6, 10, 14);
        quarter_round(&mut x, 3, 7, 11, 15);
        quarter_round(&mut x, 0, 5, 10, 15);
        quarter_round(&mut x, 1, 6, 11, 12);
        quarter_round(&mut x, 2, 7, 8, 13);
        quarter_round(&mut x, 3, 4, 9, 14);
    }
    let mut out = [0u8; 64];
    for i in 0..16 {
        let word = x[i].wrapping_add(initial[i]).to_le_bytes();
        out[i * 4..i * 4 + 4].copy_from_slice(&word);
    }
    out
}

#[inline]
fn quarter_round(x: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    x[a] = x[a].wrapping_add(x[b]);
    x[d] = (x[d] ^ x[a]).rotate_left(16);
    x[c] = x[c].wrapping_add(x[d]);
    x[b] = (x[b] ^ x[c]).rotate_left(12);
    x[a] = x[a].wrapping_add(x[b]);
    x[d] = (x[d] ^ x[a]).rotate_left(8);
    x[c] = x[c].wrapping_add(x[d]);
    x[b] = (x[b] ^ x[c]).rotate_left(7);
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;
    let mut k_block = [0u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        let digest = Sha256::digest(key);
        k_block[..32].copy_from_slice(&digest);
    } else {
        k_block[..key.len()].copy_from_slice(key);
    }

    let mut i_pad = [0x36u8; BLOCK_SIZE];
    let mut o_pad = [0x5cu8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        i_pad[i] ^= k_block[i];
        o_pad[i] ^= k_block[i];
    }

    let mut inner = Sha256::new();
    inner.update(i_pad);
    inner.update(data);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(o_pad);
    outer.update(inner_hash);
    outer.finalize().into()
}

/// RFC 8018 PBKDF2-HMAC-SHA256 for a single 32-byte block (`dkLen = 32`).
pub fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut salt_block = Vec::with_capacity(salt.len() + 4);
    salt_block.extend_from_slice(salt);
    salt_block.extend_from_slice(&1u32.to_be_bytes());

    let mut u = hmac_sha256(password, &salt_block);
    let mut out = u;
    for _ in 1..iterations {
        u = hmac_sha256(password, &u);
        for (o, b) in out.iter_mut().zip(u.iter()) {
            *o ^= *b;
        }
    }
    out
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    std::hint::black_box(diff) == 0
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn decode_hex(hex: &str) -> Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("Corrupt hex in lock file".to_string());
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    let bytes = hex.as_bytes();
    for pair in bytes.chunks_exact(2) {
        let hi = hex_nibble(pair[0])?;
        let lo = hex_nibble(pair[1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn hex_nibble(b: u8) -> Result<u8, String> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err("Corrupt hex in lock file".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pbkdf2_hmac_sha256_matches_rfc_6070_vectors() {
        let dk = pbkdf2_hmac_sha256(b"password", b"salt", 2);
        assert_eq!(encode_hex(&dk), "ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6e2d85a95474c43");
    }

    #[test]
    fn aead_seal_and_unseal_detects_tampering() {
        let key = [0x42u8; 32];
        let msg = b"SQLite format 3\0sensitive accounting data";
        let mut sealed = seal_bytes(&key, msg).unwrap();
        assert_ne!(&sealed[..], msg);
        assert_eq!(unseal_bytes(&key, &sealed).unwrap(), msg);

        // Flip a byte in ciphertext -> MAC must reject
        let mid = sealed.len() / 2;
        sealed[mid] ^= 0xff;
        assert!(unseal_bytes(&key, &sealed).is_err());
    }

    #[test]
    fn seal_and_unseal_workspace_protects_sqlite_backups_and_exports() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("sarv.sqlite3");
        let backup = dir.path().join("sarv-backup-1.json");
        let exports = dir.path().join("exports");
        fs::create_dir_all(&exports).unwrap();
        let pdf = exports.join("invoice-1.pdf");

        fs::write(&db, b"SQLite format 3\0books").unwrap();
        fs::write(&backup, br#"{"format":"v7"}"#).unwrap();
        fs::write(&pdf, b"%PDF-1.4\0invoice").unwrap();

        let dek = set_password(dir.path(), "my-pass", 1000).unwrap();
        seal_workspace_data(dir.path(), &dek).unwrap();

        assert!(!db.exists(), "plaintext sqlite must be removed while locked");
        assert!(!backup.exists(), "plaintext backup must be removed while locked");
        assert!(!pdf.exists(), "plaintext export pdf must be removed while locked");
        assert!(dir.path().join("sarv.sqlite3.sarvlock").is_file());
        assert!(dir.path().join("sarv-backup-1.json.sarvlock").is_file());
        assert!(exports.join("invoice-1.pdf.sarvlock").is_file());

        let VerifyOutcome::Unlocked(unlocked_dek) = verify_and_record(dir.path(), "my-pass", 1_700_000_000).unwrap() else {
            panic!("expected unlock");
        };
        assert_eq!(unlocked_dek, dek);
        unseal_workspace_data(dir.path(), &unlocked_dek).unwrap();

        assert_eq!(fs::read(&db).unwrap(), b"SQLite format 3\0books");
        assert_eq!(fs::read(&backup).unwrap(), br#"{"format":"v7"}"#);
        assert_eq!(fs::read(&pdf).unwrap(), b"%PDF-1.4\0invoice");
    }

    #[test]
    fn progressive_lockout_after_three_failures_and_escalates() {
        let dir = tempfile::tempdir().unwrap();
        let dek = set_password(dir.path(), "correct-horse", 1000).unwrap();
        assert!(is_enabled(dir.path()));

        let mut t = 1_700_000_000i64;

        assert_eq!(
            verify_and_record(dir.path(), "wrong1", t).unwrap(),
            VerifyOutcome::WrongPassword {
                failed_attempts: 1,
                attempts_until_lock: 2,
                lockout_secs: 0,
            }
        );

        assert_eq!(
            verify_and_record(dir.path(), "wrong2", t).unwrap(),
            VerifyOutcome::WrongPassword {
                failed_attempts: 2,
                attempts_until_lock: 1,
                lockout_secs: 0,
            }
        );

        assert_eq!(
            verify_and_record(dir.path(), "wrong3", t).unwrap(),
            VerifyOutcome::WrongPassword {
                failed_attempts: 3,
                attempts_until_lock: 0,
                lockout_secs: 30,
            }
        );

        assert_eq!(
            verify_and_record(dir.path(), "correct-horse", t + 10).unwrap(),
            VerifyOutcome::LockedOut {
                remaining_secs: 20,
                failed_attempts: 3,
            }
        );

        t += 30;
        assert_eq!(
            verify_and_record(dir.path(), "wrong4", t).unwrap(),
            VerifyOutcome::WrongPassword {
                failed_attempts: 4,
                attempts_until_lock: 0,
                lockout_secs: 60,
            }
        );

        t += 60;
        assert_eq!(
            verify_and_record(dir.path(), "wrong5", t).unwrap(),
            VerifyOutcome::WrongPassword {
                failed_attempts: 5,
                attempts_until_lock: 0,
                lockout_secs: 300,
            }
        );

        t += 300;
        assert_eq!(
            verify_and_record(dir.path(), "correct-horse", t).unwrap(),
            VerifyOutcome::Unlocked(dek)
        );
        let rec = load(dir.path()).unwrap().unwrap();
        assert_eq!(rec.failed_attempts, 0);
        assert_eq!(rec.locked_until_unix, 0);
    }

    #[test]
    fn change_and_disable_preserve_and_unseal_data() {
        let dir = tempfile::tempdir().unwrap();
        let dek = set_password(dir.path(), "pass1", 1000).unwrap();
        let t = 1_700_000_000i64;

        let wrong = change_password(dir.path(), "bad", "pass2", t, 1000).unwrap();
        assert!(matches!(wrong, VerifyOutcome::WrongPassword { .. }));

        assert_eq!(
            change_password(dir.path(), "pass1", "pass2", t, 1000).unwrap(),
            VerifyOutcome::Unlocked(dek)
        );
        assert_eq!(verify_and_record(dir.path(), "pass2", t).unwrap(), VerifyOutcome::Unlocked(dek));

        assert_eq!(disable(dir.path(), "pass2", t).unwrap(), VerifyOutcome::Unlocked(dek));
        assert!(!is_enabled(dir.path()));
    }
}
