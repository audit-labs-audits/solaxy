use std::sync::OnceLock;

use lazy_static::lazy_static;

/// Attempt to load the ELF file at the given path. If the file is empty or cannot be read,
/// a warning will be printed and the default empty vector will be returned.
fn load_elf(path: &str) -> &'static [u8] {
    static ELF: OnceLock<Vec<u8>> = OnceLock::new();
    ELF.get_or_init(|| {
        let elf = std::fs::read(path).unwrap_or_default();
        if elf.is_empty() {
            println!("Warning: ELF file at '{path}' is empty or could not be read");
        }
        elf
    })
    .as_slice()
}

const MANIFEST_PATH: &str = env!("CARGO_MANIFEST_DIR");
const PATH_TO_ELF: &str = "target/elf-compilation/riscv32im-succinct-zkvm-elf/release";

fn create_elf_path(guest_name: &str) -> String {
    format!("{MANIFEST_PATH}/guest-{guest_name}/{PATH_TO_ELF}/sov-prover-guest-{guest_name}-sp1")
}

// Initialize the SP1 guest ELFs. Note: Normally this is done with include_bytes!(PATH_TO_FILE),
// but because we don't include the guest ELFs in the GitHub build, they may potentially not exist.
lazy_static! {
    pub static ref SP1_GUEST_MOCK_ELF: &'static [u8] = load_elf(&create_elf_path("mock"));
    pub static ref SP1_GUEST_CELESTIA_ELF: &'static [u8] = load_elf(&create_elf_path("celestia"));
    pub static ref SP1_GUEST_SOLANA_ELF: &'static [u8] = load_elf(&create_elf_path("solana"));
}
