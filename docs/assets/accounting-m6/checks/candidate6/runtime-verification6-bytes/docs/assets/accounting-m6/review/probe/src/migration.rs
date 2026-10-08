use anyhow::{Result,ensure};
use sarv_quote::store::Store;
use serde_json::{Value,json};
use std::{fs,path::{Path,PathBuf},process::Command};
fn row(path:&Path)->Result<(i64,String)>{let db=rusqlite::Connection::open(path.join("sarv.sqlite3"))?;Ok(db.query_row("SELECT generation,payload FROM state WHERE id=1",[],|r|Ok((r.get(0)?,r.get(1)?)))?)}
fn main()->Result<()>{
 let out=PathBuf::from(std::env::args().nth(1).expect("fresh private output"));fs::create_dir_all(&out)?;
 let old=PathBuf::from(std::env::var("SARV_REVIEW_M5_OLD_BINARY").expect("genuine immutable archived M5 helper required"));let profile=out.join("genuine-old-m5-profile");
 let seeded=Command::new(&old).arg("--seed").arg(&profile).output()?;fs::write(out.join("old-m5-seed.log"),[seeded.stdout,seeded.stderr].concat())?;ensure!(seeded.status.success(),"genuine archived M5 seed");
 let(g,before)=row(&profile)?;let original:Value=serde_json::from_str(&before)?;ensure!(original["schema"]==6,"genuine oldM5 schema6");
 let migrated=Store::open(&profile)?;ensure!(migrated.state.schema==7&&migrated.local_generation()==g+1,"6->7 conditional single generation");
 let after=serde_json::to_value(&migrated.state)?;let mut only_schema=original.clone();only_schema["schema"]=7.into();ensure!(after==only_schema,"migration preserves complete immutable M5 State prefix except schema");
 let recovery=fs::read_dir(&profile)?.filter_map(|e|e.ok()).find(|e|e.file_name().to_string_lossy().starts_with("before-schema7-")).expect("original recovery").path();let recovered:Value=serde_json::from_slice(&fs::read(&recovery)?)?;ensure!(recovered["state"]==original&&recovered["format"]=="sarv-accounting-backup-v6","exact original recovery before migration");
 let before_refusal=row(&profile)?;let refused=Command::new(&old).arg("--open").arg(&profile).output()?;fs::write(out.join("actual-old-M5-refusal.log"),[refused.stdout,refused.stderr].concat())?;ensure!(refused.status.code()==Some(13)&&row(&profile)?==before_refusal,"actual oldM5 refuses7 preserving logical payload/generation");
 let new_backup=out.join("migrated-v7.json");migrated.backup(&new_backup)?;let mut fresh=Store::open(&out.join("fresh-restore"))?;fresh.restore(Store::restore_preview(&new_backup)?)?;ensure!(serde_json::to_value(&fresh.state)?==after,"migrated complete State fresh restore exact");
 let old_backup=profile.join("actual-old-M5-v6.json");let restored_old=Store::restore_preview(&old_backup)?;let mut old_fresh=Store::open(&out.join("fresh-v6-backup-restore"))?;old_fresh.restore(restored_old)?;ensure!(serde_json::to_value(&old_fresh.state)?==after,"actual oldM5 v6 backup restore upgrades exact prefix");
 let mut corrupt:Value=serde_json::from_slice(&fs::read(&new_backup)?)?;corrupt["checksum"]="wrong".into();let corruptfile=out.join("checksum-corrupt.json");fs::write(&corruptfile,serde_json::to_vec_pretty(&corrupt)?)?;ensure!(Store::restore_preview(&corruptfile).is_err(),"bad checksum rejected before replacement");
 fs::write(out.join("migration-result.json"),serde_json::to_vec_pretty(&json!({"genuine_old_source":true,"original_schema":6,"new_schema":7,"exact_original_recovery":true,"full_prefix_preserved":true,"old_binary_refused_without_payload_mutation":true,"full_fresh_restores":2,"checksum_rejected":true,"hardware_power_loss_tested":false}))?)?;println!("PASS genuine M5 schema6->7 original recovery/prefix, actual old writer refusal and complete fresh restores");Ok(())
}
