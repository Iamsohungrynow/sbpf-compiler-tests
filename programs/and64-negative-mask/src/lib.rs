//! A 64-bit AND with a negative mask whose result feeds a 32-bit operation.
//!
//! In SBPF `add32` sign-extends, so the linker zero-extends its result unless
//! range analysis shows bit 31 can't be set. For `and64` with a negative
//! immediate, range analysis kept the input's range, but clearing low bits can
//! take the value below the input's minimum. Here the byte sum of `a` plus one
//! is in `[1, 2041]`, `& !7` (`and64 rX, -8`) can make it 0, and the `add32`
//! computing `rounded as u32 - 1` was left sign-extended: 0 - 1 came back as
//! `0xFFFFFFFFFFFFFFFF` instead of `0xFFFFFFFF` (blueshift-gg/sbpf#194).
//!
//! All 8 bytes are summed so LLVM can't prove the upper bits are zero; for a
//! single byte it shrinks the mask to `0x1f8`. `rounded` is also added back as
//! a `u64`, so the AND stays 64-bit.
//!
//! Instruction data: `[a: u64 LE, expected: u64 LE]`.
//! The program returns 0 if `compute(a) == expected`, 1 otherwise. On a
//! mismatch it also sets the value it computed as return data (8 bytes LE), so
//! a failing test can show what the program actually produced.

#![cfg_attr(any(target_arch = "bpf", target_os = "solana"), no_std)]

#[cfg(target_arch = "bpf")]
use solana_compiler_builtins as _;

#[cfg(any(target_arch = "bpf", target_os = "solana"))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}

/// Shared by the program and the host test, so the host computes the expected
/// value with Rust's zero-extending `u32 as u64`.
#[inline(always)]
pub fn compute(a: u64) -> u64 {
    let sum = a
        .to_le_bytes()
        .iter()
        .fold(1u64, |acc, &byte| acc + byte as u64);
    let rounded = sum & !7;
    ((rounded as u32).wrapping_sub(1) as u64).wrapping_add(rounded)
}

#[cfg(any(target_arch = "bpf", target_os = "solana"))]
mod program {
    use {core::ptr::read_unaligned, solana_define_syscall::definitions::sol_set_return_data};

    #[unsafe(no_mangle)]
    extern "C" fn entrypoint(input: *mut u8) -> u64 {
        // With no accounts, instruction data starts at offset 16.
        let data = unsafe { input.add(16) } as *const u64;
        let a = unsafe { read_unaligned(data) };
        let expected = unsafe { read_unaligned(data.add(1)) };

        let actual = super::compute(a);
        if actual == expected {
            0
        } else {
            // Only on mismatch, so the success path (and its CU cost) is unchanged.
            let bytes = actual.to_le_bytes();
            unsafe { sol_set_return_data(bytes.as_ptr(), bytes.len() as u64) };
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        harness::{Instruction, Mollusk, Pubkey},
    };

    /// Byte sums 0 to 6 round down to 0, so `rounded - 1` wraps; the others
    /// round to 8 or more.
    const INPUTS: &[u64] = &[
        0,
        1,
        6,
        0x0101_0101_0101,
        0x0500_0000_0000_0001,
        7,
        8,
        0x0101_0101_0101_0101,
        0xFF,
        0x1234_5678_9ABC_DEF0,
        u64::MAX,
    ];

    fn instruction_data(a: u64, expected: u64) -> Vec<u8> {
        [a, expected]
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .collect()
    }

    #[test]
    fn test_and64_negative_mask_results_are_zero_extended() {
        let program_id: Pubkey = [2u8; 32].into();
        let mollusk = Mollusk::new(&program_id, &harness::program_elf());

        let failures: Vec<String> = INPUTS
            .iter()
            .filter_map(|&a| {
                let expected = compute(a);
                let result = mollusk.process_instruction(
                    &Instruction {
                        program_id,
                        accounts: vec![],
                        data: instruction_data(a, expected),
                    },
                    &[],
                );
                let actual = <[u8; 8]>::try_from(result.return_data.as_slice())
                    .map(|bytes| format!("{:#x}", u64::from_le_bytes(bytes)))
                    .unwrap_or_else(|_| "?".to_owned());
                result
                    .raw_result
                    .err()
                    .map(|err| format!("a {a:#x} expected {expected:#x} actual {actual}: {err:?}"))
            })
            .collect();

        assert!(
            failures.is_empty(),
            "{} of {} cases failed:\n{}",
            failures.len(),
            INPUTS.len(),
            failures.join("\n")
        );
    }
}
