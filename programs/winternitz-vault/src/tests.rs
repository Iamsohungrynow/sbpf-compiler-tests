use mollusk_svm::{program::keyed_account_for_system_program, result::Check, Mollusk};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_program_error::ProgramError;
use solana_pubkey::Pubkey;
use solana_winternitz::{privkey::WinternitzPrivkey, HASH_LENGTH};

use crate::VaultInstructions;

const BALANCE: u64 = 100_000_000;

fn program_id() -> Pubkey {
    Pubkey::new_from_array(crate::ID)
}

fn mollusk() -> Mollusk {
    Mollusk::new(&program_id(), &harness::program_elf())
}

fn key() -> WinternitzPrivkey {
    // Fixed test-only seeds keep signatures and execution costs reproducible.
    WinternitzPrivkey::from(core::array::from_fn::<_, 32, _>(|i| [i as u8; HASH_LENGTH]))
}

fn vault(key: &WinternitzPrivkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&key.pubkey().merklize()], &program_id())
}

#[test]
fn open_vault() {
    let key = key();
    let (vault, bump) = vault(&key);
    let payer = Pubkey::new_from_array([7; 32]);
    let (system, system_account) = keyed_account_for_system_program();
    let svm = mollusk();
    let rent = svm.sysvars.rent.minimum_balance(0);
    let instruction = Instruction::new_with_bytes(
        program_id(),
        &[
            &[VaultInstructions::OpenVault as u8],
            key.pubkey().merklize().as_ref(),
            &[bump],
        ]
        .concat(),
        vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(system, false),
        ],
    );
    svm.process_and_validate_instruction(
        &instruction,
        &[
            (payer, Account::new(BALANCE, 0, &system)),
            (vault, Account::default()),
            (system, system_account),
        ],
        &[
            Check::success(),
            Check::account(&vault)
                .owner(&program_id())
                .lamports(rent)
                .build(),
            Check::account(&payer).lamports(BALANCE - rent).build(),
        ],
    );
}

#[test]
fn split_vault() {
    let key = key();
    let (vault, bump) = vault(&key);
    let split = Pubkey::new_from_array([8; 32]);
    let refund = Pubkey::new_from_array([9; 32]);
    let amount = 1337u64;
    let message = [&amount.to_le_bytes()[..], split.as_ref(), refund.as_ref()].concat();
    let signature: [u8; HASH_LENGTH * 32] = key.sign(&message).into();
    let instruction = Instruction::new_with_bytes(
        program_id(),
        &[
            &[VaultInstructions::SplitVault as u8],
            signature.as_ref(),
            &amount.to_le_bytes(),
            &[bump],
        ]
        .concat(),
        vec![
            AccountMeta::new(vault, false),
            AccountMeta::new(split, false),
            AccountMeta::new(refund, false),
        ],
    );
    mollusk().process_and_validate_instruction(
        &instruction,
        &[
            (vault, Account::new(BALANCE, 0, &program_id())),
            (split, Account::default()),
            (refund, Account::default()),
        ],
        &[
            Check::success(),
            Check::account(&vault)
                .lamports(0)
                .owner(&Pubkey::default())
                .build(),
            Check::account(&split).lamports(amount).build(),
            Check::account(&refund).lamports(BALANCE - amount).build(),
        ],
    );
}

fn close_fixture(tamper: bool) -> (Instruction, Vec<(Pubkey, Account)>, Pubkey, Pubkey) {
    let key = key();
    let (vault, bump) = vault(&key);
    let refund = Pubkey::new_from_array([10; 32]);
    let mut signature: [u8; HASH_LENGTH * 32] = key.sign(refund.as_ref()).into();
    if tamper {
        signature[0] ^= 1;
    }
    let instruction = Instruction::new_with_bytes(
        program_id(),
        &[
            &[VaultInstructions::CloseVault as u8],
            signature.as_ref(),
            &[bump],
        ]
        .concat(),
        vec![
            AccountMeta::new(vault, false),
            AccountMeta::new(refund, true),
        ],
    );
    let accounts = vec![
        (vault, Account::new(BALANCE, 0, &program_id())),
        (refund, Account::new(BALANCE, 0, &Pubkey::default())),
    ];
    (instruction, accounts, vault, refund)
}

#[test]
fn close_vault() {
    let (instruction, accounts, vault, refund) = close_fixture(false);
    mollusk().process_and_validate_instruction(
        &instruction,
        &accounts,
        &[
            Check::success(),
            Check::account(&vault)
                .lamports(0)
                .owner(&Pubkey::default())
                .build(),
            Check::account(&refund).lamports(BALANCE * 2).build(),
        ],
    );
}

#[test]
fn reject_invalid_signature() {
    let (instruction, accounts, vault, refund) = close_fixture(true);
    mollusk().process_and_validate_instruction(
        &instruction,
        &accounts,
        &[
            Check::err(ProgramError::MissingRequiredSignature),
            Check::account(&vault)
                .lamports(BALANCE)
                .owner(&program_id())
                .build(),
            Check::account(&refund).lamports(BALANCE).build(),
        ],
    );
}

#[test]
fn reject_invalid_instruction_data() {
    let svm = mollusk();
    for data in [vec![], vec![3], vec![0], vec![1], vec![2]] {
        svm.process_and_validate_instruction(
            &Instruction::new_with_bytes(program_id(), &data, vec![]),
            &[],
            &[Check::err(ProgramError::InvalidInstructionData)],
        );
    }
}
