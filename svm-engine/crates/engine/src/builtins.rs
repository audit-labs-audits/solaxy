use solana_program_runtime::invoke_context::BuiltinFunctionWithContext;
use solana_sdk::{bpf_loader, bpf_loader_upgradeable, compute_budget, pubkey::Pubkey};

/// A built-in program to include in the SVM.
pub struct Builtin {
    pub name: &'static str,
    pub id: Pubkey,
    pub entrypoint: BuiltinFunctionWithContext,
}

/// List of built-in programs to include in the SVM at all times.
pub const BUILTINS: &[Builtin] = &[
    Builtin {
        name: "system_program",
        id: solana_system_program::id(),
        entrypoint: solana_system_program::system_processor::Entrypoint::vm,
    },
    Builtin {
        name: "solana_bpf_loader_program",
        id: bpf_loader::id(),
        entrypoint: solana_bpf_loader_program::Entrypoint::vm,
    },
    Builtin {
        name: "solana_bpf_loader_upgradeable_program",
        id: bpf_loader_upgradeable::id(),
        entrypoint: solana_bpf_loader_program::Entrypoint::vm,
    },
    Builtin {
        name: "compute_budget_program",
        id: compute_budget::id(),
        entrypoint: solana_compute_budget_program::Entrypoint::vm,
    },
];
