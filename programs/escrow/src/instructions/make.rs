use pinocchio::{
    ProgramResult,
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    log::sol_log,
    memory::sol_memcpy,
    program_error::ProgramError,
    pubkey::find_program_address,
    sysvars::{Sysvar, rent::Rent},
};
use pinocchio_associated_token_account::instructions::CreateIdempotent;
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::instructions::Transfer;

use crate::{
    constants::ESCROW_SEED,
    state::{Context, EscrowAccount},
};

#[derive(Debug)]
pub struct MakeAccounts<'info> {
    maker: &'info AccountInfo,
    mint_a: &'info AccountInfo,
    mint_b: &'info AccountInfo,
    maker_token_a: &'info AccountInfo,
    escrow: &'info AccountInfo,
    vault: &'info AccountInfo,
    token_program: &'info AccountInfo,
    _associated_token_program: &'info AccountInfo,
    system_program: &'info AccountInfo,
}

impl<'info> TryFrom<&'info [AccountInfo]> for MakeAccounts<'info> {
    type Error = ProgramError;

    fn try_from(accounts: &'info [AccountInfo]) -> Result<Self, Self::Error> {
        let [
            maker,
            mint_a,
            mint_b,
            maker_token_a,
            escrow,
            vault,
            token_program,
            _associated_token_program,
            system_program,
        ] = accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // Account Checks
        if !maker.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if !vault.is_writable() {
            sol_log("Vault account is not mutable");
            return Err(ProgramError::InvalidAccountData);
        }

        if !escrow.is_writable() {
            sol_log("Escrow account is not mutable");
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self {
            maker,
            mint_a,
            mint_b,
            maker_token_a,
            escrow,
            vault,
            token_program,
            _associated_token_program,
            system_program,
        })
    }
}

pub struct Make<'info> {
    accounts: MakeAccounts<'info>,
    amount_in: u64,
    amount_out: u64,
}

impl<'info> TryFrom<Context<'info>> for Make<'info> {
    type Error = ProgramError;

    fn try_from(ctx: Context<'info>) -> Result<Self, Self::Error> {
        let accounts = MakeAccounts::try_from(ctx.accounts)?;

        if ctx
            .instruction_data
            .len()
            .ne(&(core::mem::size_of::<u64>() * 2))
        {
            return Err(ProgramError::InvalidInstructionData);
        }

        let amount_in = u64::from_le_bytes(
            ctx.instruction_data[..8]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        );
        let amount_out = u64::from_le_bytes(
            ctx.instruction_data[8..16]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        );

        Ok(Self {
            accounts,
            amount_in,
            amount_out,
        })
    }
}

impl<'info> Make<'info> {
    pub fn execute(&self) -> ProgramResult {
        sol_log("Make invoked");

        let space = core::mem::size_of::<EscrowAccount>();
        let lamports = Rent::get()?.minimum_balance(space);

        let seeds: [&[u8]; 4] = [
            b"escrow".as_ref(),
            self.accounts.maker.key().as_ref(),
            self.accounts.mint_a.key().as_ref(),
            self.accounts.mint_b.key().as_ref(),
        ];

        let bump = find_program_address(&seeds, &crate::ID).1;

        let bump = [bump];

        let seeds = [
            Seed::from(ESCROW_SEED),
            Seed::from(self.accounts.maker.key().as_ref()),
            Seed::from(self.accounts.mint_a.key().as_ref()),
            Seed::from(self.accounts.mint_b.key().as_ref()),
            Seed::from(&bump),
        ];
        let signers: [Signer; 1] = [Signer::from(&seeds)];

        // Initialize our escrow account
        CreateAccount {
            from: self.accounts.maker,
            to: self.accounts.escrow,
            lamports,
            space: space as u64,
            owner: &crate::ID,
        }
        .invoke_signed(&signers)?;

        // Create vault account
        CreateIdempotent {
            funding_account: self.accounts.maker,
            account: self.accounts.vault,
            wallet: self.accounts.escrow,
            mint: self.accounts.mint_a,
            system_program: self.accounts.system_program,
            token_program: self.accounts.token_program,
        }
        .invoke_signed(&signers)?;

        // Deposit initial capital to vault
        Transfer {
            from: self.accounts.maker_token_a,
            to: self.accounts.vault,
            authority: self.accounts.maker,
            amount: self.amount_in,
        }
        .invoke()?;

        // Set our escrow account to the correct values
        let mut escrow_account_ref = self.accounts.escrow.try_borrow_mut_data()?;
        let escrow_account: &mut EscrowAccount = escrow_account_ref.as_mut().try_into()?;
        unsafe { sol_memcpy(&mut escrow_account.maker, self.accounts.maker.key(), 32) };
        unsafe { sol_memcpy(&mut escrow_account.mint_a, self.accounts.mint_a.key(), 32) };
        unsafe { sol_memcpy(&mut escrow_account.mint_b, self.accounts.mint_b.key(), 32) };
        escrow_account.amount_out = self.amount_out;
        escrow_account.bump = bump;

        Ok(())
    }
}
