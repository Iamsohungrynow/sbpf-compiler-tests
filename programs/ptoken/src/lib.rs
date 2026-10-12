//! pToken program snapshot with host-side Mollusk tests.
#![cfg_attr(any(target_arch = "bpf", target_os = "solana"), no_std)]

#[cfg(target_arch = "bpf")]
use solana_compiler_builtins as _;

#[cfg(any(target_arch = "bpf", target_os = "solana"))]
mod entrypoint;
#[cfg(any(target_arch = "bpf", target_os = "solana"))]
mod processor;
