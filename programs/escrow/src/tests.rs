use mollusk_svm::{Mollusk, program::keyed_account_for_system_program, result::Check};
use mollusk_svm_programs_token::{associated_token, token};
use solana_account::Account;
use solana_instruction::{AccountMeta, Instruction};
use solana_program_pack::Pack;
use solana_pubkey::Pubkey as Address;

use crate::constants::ESCROW_SEED;

#[test]
pub fn make_and_take() {
    let mut mollusk = Mollusk::new(&crate::ID.into(), &harness::program_elf());

    let maker = Address::new_unique();
    let taker = Address::new_unique();
    let mint_a = Address::new_unique();
    let mint_b = Address::new_unique();

    let mint_a_data = spl_token_interface::state::Mint {
        mint_authority: None.into(),
        supply: 1_000_000_000_000_000,
        decimals: 6,
        is_initialized: true,
        freeze_authority: None.into(),
    };

    let mint_b_data = spl_token_interface::state::Mint {
        mint_authority: None.into(),
        supply: 1_000_000_000_000_000,
        decimals: 6,
        is_initialized: true,
        freeze_authority: None.into(),
    };

    let maker_token_a_data = spl_token_interface::state::Account {
        mint: mint_a.to_bytes().into(),
        owner: maker.to_bytes().into(),
        amount: 100_000_000,
        delegate: None.into(),
        state: spl_token_interface::state::AccountState::Initialized,
        is_native: None.into(),
        delegated_amount: 0,
        close_authority: None.into(),
    };

    let taker_token_b_data = spl_token_interface::state::Account {
        mint: mint_b.to_bytes().into(),
        owner: taker.to_bytes().into(),
        amount: 100_000_000,
        delegate: None.into(),
        state: spl_token_interface::state::AccountState::Initialized,
        is_native: None.into(),
        delegated_amount: 0,
        close_authority: None.into(),
    };

    let (maker_token_a, maker_token_a_account) =
        associated_token::create_account_for_associated_token_account(maker_token_a_data);
    let (taker_token_b, taker_token_b_account) =
        associated_token::create_account_for_associated_token_account(taker_token_b_data);

    let mut mint_a_data_bytes = vec![0u8; spl_token_interface::state::Mint::LEN];
    let mut mint_b_data_bytes = vec![0u8; spl_token_interface::state::Mint::LEN];

    mint_a_data.pack_into_slice(&mut mint_a_data_bytes);
    mint_b_data.pack_into_slice(&mut mint_b_data_bytes);

    let maker_account = Account::new(10_000_000_000, 0, &Address::default());
    let taker_account = Account::new(10_000_000_000, 0, &Address::default());
    let mint_a_account = Account {
        lamports: mollusk
            .sysvars
            .rent
            .minimum_balance(spl_token_interface::state::Mint::LEN),
        data: mint_a_data_bytes,
        owner: token::ID,
        executable: false,
        rent_epoch: 0,
    };

    let mint_b_account = Account {
        lamports: mollusk
            .sysvars
            .rent
            .minimum_balance(spl_token_interface::state::Mint::LEN),
        data: mint_b_data_bytes,
        owner: token::ID,
        executable: false,
        rent_epoch: 0,
    };

    let (system_program, system_program_account) = keyed_account_for_system_program();
    let (token_program, token_program_account) = token::keyed_account();
    let (associated_token_program, associated_token_program_account) =
        associated_token::keyed_account();

    let seeds: &[&[u8]] = &[
        ESCROW_SEED.as_ref(),
        maker.as_ref(),
        mint_a.as_ref(),
        mint_b.as_ref(),
    ];
    let escrow = Address::find_program_address(seeds, &crate::ID.into()).0;
    let escrow_account = Account::default();

    let vault = spl_associated_token_account_interface::address::get_associated_token_address(
        &escrow, &mint_a,
    );
    let vault_account = Account::default();

    let maker_token_b =
        spl_associated_token_account_interface::address::get_associated_token_address(
            &maker, &mint_b,
        );
    let maker_token_b_account = Account::default();
    let taker_token_a =
        spl_associated_token_account_interface::address::get_associated_token_address(
            &taker, &mint_a,
        );
    let taker_token_a_account = Account::default();

    token::add_program(&mut mollusk);
    associated_token::add_program(&mut mollusk);

    mollusk.process_and_validate_instruction_chain(
        &[
            (
                &Instruction {
                    program_id: crate::ID.into(),
                    accounts: vec![
                        AccountMeta::new(maker, true),
                        AccountMeta::new_readonly(mint_a, false),
                        AccountMeta::new_readonly(mint_b, false),
                        AccountMeta::new(maker_token_a, false),
                        AccountMeta::new(escrow, false),
                        AccountMeta::new(vault, false),
                        AccountMeta::new_readonly(token_program, false),
                        AccountMeta::new_readonly(associated_token_program, false),
                        AccountMeta::new_readonly(system_program, false),
                    ],
                    data: vec![0, 13, 37, 0, 0, 0, 0, 0, 0, 13, 37, 0, 0, 0, 0, 0, 0],
                },
                &[
                    // Check tests all passed
                    Check::success(),
                    // Check vault is owned by token program
                    Check::account(&vault).owner(&token_program).build(),
                    // Check escrow is owned by our program
                    Check::account(&escrow).owner(&crate::ID.into()).build(),
                ],
            ),
            (
                &Instruction {
                    program_id: crate::ID.into(),
                    accounts: vec![
                        AccountMeta::new(taker, true),
                        AccountMeta::new(maker, false),
                        AccountMeta::new_readonly(mint_a, false),
                        AccountMeta::new_readonly(mint_b, false),
                        AccountMeta::new(maker_token_b, false),
                        AccountMeta::new(taker_token_a, false),
                        AccountMeta::new(taker_token_b, false),
                        AccountMeta::new(escrow, false),
                        AccountMeta::new(vault, false),
                        AccountMeta::new_readonly(token_program, false),
                        AccountMeta::new_readonly(associated_token_program, false),
                        AccountMeta::new_readonly(system_program, false),
                    ],
                    data: vec![1],
                },
                &[
                    // Check tests all passed
                    Check::success(),
                    // Check that our vault is closed
                    Check::account(&vault).owner(&system_program).build(),
                    Check::account(&vault).lamports(0).build(),
                    // Check that our escrow is closed
                    Check::account(&escrow).owner(&system_program).build(),
                    Check::account(&escrow).lamports(0).build(),
                ],
            ),
        ],
        &vec![
            (maker, maker_account),
            (taker, taker_account),
            (mint_a, mint_a_account),
            (mint_b, mint_b_account),
            (maker_token_a, maker_token_a_account),
            (maker_token_b, maker_token_b_account),
            (taker_token_a, taker_token_a_account),
            (taker_token_b, taker_token_b_account),
            (escrow, escrow_account),
            (vault, vault_account),
            (token_program, token_program_account),
            (associated_token_program, associated_token_program_account),
            (system_program, system_program_account),
        ],
    );
}

#[test]
pub fn make_and_refund() {
    let mut mollusk = Mollusk::new(&crate::ID.into(), &harness::program_elf());

    let maker = Address::new_unique();
    let mint_a = Address::new_unique();
    let mint_b = Address::new_unique();

    let mint_a_data = spl_token_interface::state::Mint {
        mint_authority: None.into(),
        supply: 1_000_000_000_000_000,
        decimals: 6,
        is_initialized: true,
        freeze_authority: None.into(),
    };

    let mint_b_data = spl_token_interface::state::Mint {
        mint_authority: None.into(),
        supply: 1_000_000_000_000_000,
        decimals: 6,
        is_initialized: true,
        freeze_authority: None.into(),
    };

    let maker_token_a_data = spl_token_interface::state::Account {
        mint: mint_a.to_bytes().into(),
        owner: maker.to_bytes().into(),
        amount: 100_000_000,
        delegate: None.into(),
        state: spl_token_interface::state::AccountState::Initialized,
        is_native: None.into(),
        delegated_amount: 0,
        close_authority: None.into(),
    };

    let (maker_token_a, maker_token_a_account) =
        associated_token::create_account_for_associated_token_account(maker_token_a_data);

    let mut mint_a_data_bytes = vec![0u8; spl_token_interface::state::Mint::LEN];
    let mut mint_b_data_bytes = vec![0u8; spl_token_interface::state::Mint::LEN];

    mint_a_data.pack_into_slice(&mut mint_a_data_bytes);
    mint_b_data.pack_into_slice(&mut mint_b_data_bytes);

    let maker_account = Account::new(10_000_000_000, 0, &Address::default());
    let mint_a_account = Account {
        lamports: mollusk
            .sysvars
            .rent
            .minimum_balance(spl_token_interface::state::Mint::LEN),
        data: mint_a_data_bytes,
        owner: token::ID,
        executable: false,
        rent_epoch: 0,
    };

    let mint_b_account = Account {
        lamports: mollusk
            .sysvars
            .rent
            .minimum_balance(spl_token_interface::state::Mint::LEN),
        data: mint_b_data_bytes,
        owner: token::ID,
        executable: false,
        rent_epoch: 0,
    };

    let (system_program, system_program_account) = keyed_account_for_system_program();
    let (token_program, token_program_account) = token::keyed_account();
    let (associated_token_program, associated_token_program_account) =
        associated_token::keyed_account();

    let seeds: &[&[u8]] = &[
        ESCROW_SEED.as_ref(),
        maker.as_ref(),
        mint_a.as_ref(),
        mint_b.as_ref(),
    ];
    let escrow = Address::find_program_address(seeds, &crate::ID.into()).0;
    let escrow_account = Account::default();

    let vault = spl_associated_token_account_interface::address::get_associated_token_address(
        &escrow, &mint_a,
    );
    let vault_account = Account::default();

    token::add_program(&mut mollusk);
    associated_token::add_program(&mut mollusk);

    mollusk.process_and_validate_instruction_chain(
        &[
            (
                &Instruction {
                    program_id: crate::ID.into(),
                    accounts: vec![
                        AccountMeta::new(maker, true),
                        AccountMeta::new_readonly(mint_a, false),
                        AccountMeta::new_readonly(mint_b, false),
                        AccountMeta::new(maker_token_a, false),
                        AccountMeta::new(escrow, false),
                        AccountMeta::new(vault, false),
                        AccountMeta::new_readonly(token_program, false),
                        AccountMeta::new_readonly(associated_token_program, false),
                        AccountMeta::new_readonly(system_program, false),
                    ],
                    data: vec![0, 13, 37, 0, 0, 0, 0, 0, 0, 13, 37, 0, 0, 0, 0, 0, 0],
                },
                &[
                    // Check tests all passed
                    Check::success(),
                    // Check vault is owned by token program
                    Check::account(&vault).owner(&token_program).build(),
                    // Check vault balance matches
                    Check::account(&vault)
                        .data_slice(64, &[13, 37, 0, 0, 0, 0, 0, 0])
                        .build(),
                    // Check escrow is owned by our program
                    Check::account(&escrow).owner(&crate::ID.into()).build(),
                    // Check escrow amount_out matches
                    Check::account(&escrow)
                        .data_slice(96, &[13, 37, 0, 0, 0, 0, 0, 0])
                        .build(),
                ],
            ),
            (
                &Instruction {
                    program_id: crate::ID.into(),
                    accounts: vec![
                        AccountMeta::new(maker, true),
                        AccountMeta::new(maker_token_a, false),
                        AccountMeta::new(escrow, false),
                        AccountMeta::new(vault, false),
                        AccountMeta::new_readonly(token_program, false),
                        AccountMeta::new_readonly(system_program, false),
                    ],
                    data: vec![2],
                },
                &[
                    // Check tests all passed
                    Check::success(),
                    // Check that our vault is closed
                    Check::account(&vault).owner(&system_program).build(),
                    Check::account(&vault).lamports(0).build(),
                    // Check that our escrow is closed
                    Check::account(&escrow).owner(&system_program).build(),
                    Check::account(&escrow).lamports(0).build(),
                ],
            ),
        ],
        &vec![
            (maker, maker_account),
            (mint_a, mint_a_account),
            (mint_b, mint_b_account),
            (maker_token_a, maker_token_a_account),
            (escrow, escrow_account),
            (vault, vault_account),
            (token_program, token_program_account),
            (associated_token_program, associated_token_program_account),
            (system_program, system_program_account),
        ],
    );
}
