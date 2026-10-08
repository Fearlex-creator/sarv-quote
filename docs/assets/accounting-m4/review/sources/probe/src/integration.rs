#[allow(dead_code)] #[path="main.rs"] mod fixture;
use anyhow::{Result,ensure};
use sarv_quote::{store::Store,domain::State};
use std::{fs,path::PathBuf};
fn same(a:&State,b:&State)->Result<bool>{Ok(serde_json::to_value(a)?==serde_json::to_value(b)?)}
fn main()->Result<()> {
 let out=PathBuf::from(std::env::args().nth(1).unwrap());fs::create_dir_all(&out)?;
 let (base,o)=fixture::base()?;let mut owner=Store::open(&out.join("owner"))?;owner.change(|x|{x.accounting=base.clone();Ok(())})?;
 let prefix=owner.state.clone();let po=fixture::order(&o,"integration-po","2026-03-21","piece",12000,15,180)?;
 let (generation,mut candidate)=owner.begin_detached()?;
 ensure!(owner.snapshot().is_err()&&owner.backup(&out.join("pending-backup.json")).is_err()&&owner.change(|_|Ok(())).is_err(),"pending owner excludes stale export and changes");
 candidate.accounting.post_procurement_command(po.clone())?;let prepared=Store::prepare_candidate(generation,candidate.clone())?;
 let committed=Store::commit_detached(out.join("owner"),prepared)?;
 ensure!(same(&owner.state,&prefix)?&&owner.local_generation()==generation,"owner unchanged until receipt adoption");
 owner.finish_detached(Ok(committed))?;ensure!(same(&owner.state,&candidate)?&&owner.local_generation()==generation+1,"adoption exact validated candidate/generation");
 println!("PASS detached reservation/commit/adoption: owner old until durable receipt, exact candidate after adoption");
 let complete=owner.state.clone();let complete_generation=owner.local_generation();prefix.validate()?;
 ensure!(owner.change(|x|{*x=prefix.clone();Ok(())}).is_err()&&same(&owner.state,&complete)?&&owner.local_generation()==complete_generation,"valid prior state cannot delete immutable procurement through change");
 let rollback=Store::prepare_candidate(complete_generation,prefix.clone())?;
 ensure!(Store::commit_detached(out.join("owner"),rollback).is_err()&&same(&Store::open(&out.join("owner"))?.state,&complete)?,"detached transition rejects valid history rollback");
 println!("PASS both change/detached commits reject internally valid history rollback with exact unchanged persisted owner");
 let id=complete.accounting.procurement.records.last().unwrap().id.clone();let receipt=fixture::receive(&o,"integration-receipt","2026-03-22",&id,"integration-po-line","piece",5000,75)?;
 let (generation,mut next)=owner.begin_detached()?;next.accounting.post_procurement_command(receipt)?;
 let committed=Store::commit_detached(out.join("owner"),Store::prepare_candidate(generation,next.clone())?)?;
 let mut other=Store::open(&out.join("other"))?;let other_before=other.state.clone();let other_generation=other.local_generation();
 ensure!(other.adopt_committed(committed).is_err()&&same(&other.state,&other_before)?&&other.local_generation()==other_generation,"crossprofile receipt cannot overwrite owner");
 ensure!(owner.finish_detached(Err(anyhow::anyhow!("independent simulated lost receipt"))).is_err()&&same(&owner.state,&complete)?,"lost receipt leaves owner prior acknowledged snapshot");
 let reopened=Store::open(&out.join("owner"))?;ensure!(same(&reopened.state,&next)?,"durable complete state recovered after lost adoption");fixture::checked(&reopened.state.accounting,&o,"2026-03-22",(29000,315),(6000,60),149,125,1326)?;
 println!("PASS crossprofile adoption rejects; lost owner receipt reopens complete inventory375/GRNI125");
 let mut forged=serde_json::to_value(&next)?;forged["accounting"]["procurement"]["records"][1]["sequence"]=9999.into();let forged:State=serde_json::from_value(forged)?;
 ensure!(Store::prepare_candidate(reopened.local_generation(),forged).is_err(),"full replay validates before prepared receipt construction");
 let mut audit=next.clone();let event=audit.accounting.audit.iter_mut().find(|a|a.operation==sarv_quote::accounting::AuditOperation::ProcurementRecorded).unwrap();event.actor="Forged independent actor".into();
 ensure!(audit.validate().is_err(),"typed nonfinancial source audit mismatch rejects");
 println!("PASS candidate preparation rejects forged derived sequence; typed source actor/audit mismatch rejects");
 Ok(())
}
