use solana_sdk::{
    account::Account,
    bpf_loader,
    bpf_loader_upgradeable::{self},
    pubkey::Pubkey,
    rent::Rent,
};
use svm_test_utils::programs::bpf_loader_upgradeable_program_accounts;

use crate::storage::SVMStorage;

#[derive(Debug, Clone)]
pub struct Program {
    pub program_id: Pubkey,
    pub owner_program: Pubkey,
    pub bytecode: &'static [u8],
}

impl Program {
    fn create_account(&self, owner: Pubkey, executable: bool, rent: &Rent) -> Account {
        Account {
            lamports: rent.minimum_balance(self.bytecode.len()),
            data: self.bytecode.to_vec(),
            owner,
            executable,
            rent_epoch: 0,
        }
    }
}

pub fn initialize_programs<Storage: SVMStorage>(
    db: &Storage,
    programs: &[Program],
) -> eyre::Result<()> {
    for program in programs {
        let bytecode = program.bytecode.to_vec();
        // This is a V2 Loader program
        if bpf_loader::check_id(&program.owner_program) {
            db.set_account(
                &program.program_id,
                program.create_account(bpf_loader::id(), true, &Rent::default()),
            )?;
            db.set_owner_index(&bpf_loader::id(), &program.program_id)?;
        // This is a V3 Loader program
        } else if bpf_loader_upgradeable::check_id(&program.owner_program) {
            let v3_accounts = bpf_loader_upgradeable_program_accounts(
                &program.program_id,
                &bytecode,
                &Rent::default(),
            );

            for (pubkey, account) in v3_accounts {
                db.set_account(&pubkey, account)?;
            }
            db.set_owner_index(&bpf_loader_upgradeable::id(), &program.program_id)?;
        }
    }

    Ok(())
}
