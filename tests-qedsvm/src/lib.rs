//! Differential test harness: every fixture runs through both
//! `mollusk_svm::Mollusk` (agave-backed) and `qedsvm::Svm` (Lean
//! reference) and asserts byte+CU equality.
//!
//! Isolated from the main `tests/` crate to avoid solana-account
//! 3.x ↔ 4.x version conflicts.
//!
//! Note: qedsvm + agave types live in different account crates. We
//! build both account sets in parallel and convert between them at
//! the boundary. See `mollusk_to_qedsvm` below.

use mollusk_svm::Mollusk;
use qedsvm::Svm;
use mollusk_account::Account as MolluskAccountInner;
use solana_account::{AccountSharedData, ReadableAccount, WritableAccount};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

pub type MolluskAccount = MolluskAccountInner;

#[derive(Debug, Clone)]
pub struct EngineRun {
    pub cu: u64,
    pub return_data: Vec<u8>,
    /// (pubkey, account.data) for each account after execution.
    pub final_accounts: Vec<(Pubkey, Vec<u8>)>,
    pub status: String,
}

pub struct DiffFixture<'a> {
    pub name: &'static str,
    pub program_id: Pubkey,
    pub program_so_path: &'a str,
    pub program_so_bytes: &'a [u8],
    pub instruction: Instruction,
    pub mollusk_accounts: Vec<(Pubkey, MolluskAccount)>,
    pub qedsvm_accounts: Vec<(Pubkey, AccountSharedData)>,
}

pub fn run_mollusk(f: &DiffFixture) -> EngineRun {
    let mut mollusk = Mollusk::new(&f.program_id, f.program_so_path);
    let r = mollusk.process_instruction(&f.instruction, &f.mollusk_accounts);
    EngineRun {
        cu: r.compute_units_consumed,
        return_data: r.return_data.clone(),
        final_accounts: r
            .resulting_accounts
            .iter()
            .map(|(k, a)| (*k, a.data.clone()))
            .collect(),
        status: format_status(&r.program_result),
    }
}

pub fn run_qedsvm(f: &DiffFixture) -> EngineRun {
    let mut svm = Svm::default().with_cu_budget(1_400_000);
    svm.add_program(&f.program_id, f.program_so_bytes);
    let r = svm
        .process_instruction(&f.instruction, &f.qedsvm_accounts)
        .expect("qedsvm runs");
    EngineRun {
        cu: r.compute_units_consumed,
        return_data: r.return_data.clone(),
        final_accounts: r
            .resulting_accounts
            .iter()
            .map(|(k, a)| (*k, a.data().to_vec()))
            .collect(),
        status: format_status(&r.program_result),
    }
}

fn format_status<T: std::fmt::Debug>(r: T) -> String {
    let s = format!("{r:?}");
    if s.starts_with("Success") {
        "Success".into()
    } else {
        s
    }
}

/// Convert a (pubkey, mollusk_account) list into the
/// (pubkey, AccountSharedData) shape qedsvm expects. Field-by-field
/// copy — both sides use the same byte semantics, just different
/// crate types.
pub fn mollusk_to_qedsvm(
    accounts: &[(Pubkey, MolluskAccount)],
) -> Vec<(Pubkey, AccountSharedData)> {
    accounts
        .iter()
        .map(|(k, a)| {
            let mut shared = AccountSharedData::new(a.lamports, a.data.len(), &a.owner);
            shared.set_data_from_slice(&a.data);
            shared.set_executable(a.executable);
            shared.set_rent_epoch(a.rent_epoch);
            (*k, shared)
        })
        .collect()
}

/// Compare two runs and panic with a pretty diff if they differ.
/// Returns `Ok(())` on match so callers can also use it as an assertion
/// inside `#[test]` functions.
pub fn assert_runs_equal(
    label: &str,
    mollusk: &EngineRun,
    qedsvm: &EngineRun,
) -> Result<(), String> {
    let mut failures: Vec<String> = Vec::new();
    if mollusk.status != qedsvm.status {
        failures.push(format!(
            "status differ:\n    mollusk = {}\n    qedsvm  = {}",
            mollusk.status, qedsvm.status
        ));
    }
    if mollusk.cu != qedsvm.cu {
        failures.push(format!(
            "CU differ: mollusk={} qedsvm={} (Δ={})",
            mollusk.cu,
            qedsvm.cu,
            (qedsvm.cu as i64) - (mollusk.cu as i64),
        ));
    }
    if mollusk.return_data != qedsvm.return_data {
        failures.push(format!(
            "return data differ: mollusk={:x?} qedsvm={:x?}",
            mollusk.return_data, qedsvm.return_data
        ));
    }
    if mollusk.final_accounts.len() != qedsvm.final_accounts.len() {
        failures.push(format!(
            "account count differ: mollusk={} qedsvm={}",
            mollusk.final_accounts.len(),
            qedsvm.final_accounts.len()
        ));
    } else {
        for (i, ((ka, da), (kb, db))) in mollusk
            .final_accounts
            .iter()
            .zip(qedsvm.final_accounts.iter())
            .enumerate()
        {
            if ka != kb {
                failures.push(format!(
                    "account[{i}] pubkey differ: mollusk={ka} qedsvm={kb}"
                ));
            }
            if da != db {
                failures.push(format!(
                    "account[{i}] data differ (mollusk={} bytes, qedsvm={} bytes)",
                    da.len(),
                    db.len()
                ));
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        let msg = format!(
            "[{label}] divergence between mollusk and qedsvm:\n  · {}",
            failures.join("\n  · ")
        );
        Err(msg)
    }
}

/// Print a compact summary of a successful match.
pub fn print_match(label: &str, run: &EngineRun) {
    println!(
        "  ✓ {label:30} status={:8} cu={:6} accounts={:2} return={}B",
        run.status,
        run.cu,
        run.final_accounts.len(),
        run.return_data.len(),
    );
}
