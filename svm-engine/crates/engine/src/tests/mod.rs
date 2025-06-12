mod close_empty;
mod deploy;
mod sol;
mod spl;

pub use close_empty::run_close_empty_scenario;
pub use deploy::{close_program_scenario, run_deploy_and_call_scenario};
pub use sol::run_sol_scenario;
use solana_sdk::signature::{Keypair, keypair_from_seed};
pub use spl::run_spl_scenario;

pub fn generate_keypairs(n: u8) -> Vec<Keypair> {
    (0..n)
        .map(|i| {
            let mut seed = b"an_example_fixed_seed_for_testi".to_vec();
            seed.push(i);
            keypair_from_seed(&seed).unwrap()
        })
        .collect()
}

fn setup_logging() {
    if std::env::var("SOLANA_LOG").is_ok() {
        solana_logger::setup_with_default(
            r##"
            solana_rbpf::vm=debug,\
            solana_runtime::message_processor=debug,\
            solana_runtime::system_instruction_processor=trace,\
            solana_program_test=info
            "##,
        );
    }
}

#[test]
fn sol() {
    setup_logging();
    let res = run_sol_scenario();
    assert!(res.is_ok(), "{res:#?}");
}

#[test]
fn close_empty() {
    setup_logging();
    let res = run_close_empty_scenario();
    assert!(res.is_ok(), "{res:#?}");
}

#[test]
fn spl() {
    setup_logging();
    let res = run_spl_scenario();
    assert!(res.is_ok(), "{res:#?}");
}

#[test]
fn deploy_and_call() {
    setup_logging();
    let res = run_deploy_and_call_scenario();
    assert!(res.is_ok(), "{res:#?}");
}

#[test]
fn close_program() {
    setup_logging();
    let res = close_program_scenario();
    assert!(res.is_ok(), "{res:#?}");
}
