#![allow(unsafe_op_in_unsafe_fn)]
use pinocchio::{
    ProgramResult,
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    log::sol_log,
    program_error::ProgramError,
};
use pinocchio_associated_token_account::instructions::CreateIdempotent;
use pinocchio_token::{
    instructions::{CloseAccount, Transfer},
    state::TokenAccount,
};

use crate::{
    constants::ESCROW_SEED,
    state::{Context, EscrowAccount},
};

pub struct TakeAccounts<'info> {
    taker: &'info AccountInfo,
    maker: &'info AccountInfo,
    mint_a: &'info AccountInfo,
    mint_b: &'info AccountInfo,
    maker_token_b: &'info AccountInfo,
    taker_token_a: &'info AccountInfo,
    taker_token_b: &'info AccountInfo,
    escrow: &'info AccountInfo,
    vault: &'info AccountInfo,
    token_program: &'info AccountInfo,
    _associated_token_program: &'info AccountInfo,
    system_program: &'info AccountInfo,
}

impl<'info> TryFrom<&'info [AccountInfo]> for TakeAccounts<'info> {
    type Error = ProgramError;
    #[inline(always)]
    fn try_from(accounts: &'info [AccountInfo]) -> Result<Self, Self::Error> {
        let [
            taker,
            maker,
            mint_a,
            mint_b,
            maker_token_b,
            taker_token_a,
            taker_token_b,
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
        if !taker.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            taker,
            maker,
            mint_a,
            mint_b,
            maker_token_b,
            taker_token_a,
            taker_token_b,
            escrow,
            vault,
            token_program,
            _associated_token_program,
            system_program,
        })
    }
}

pub struct Take<'info> {
    accounts: TakeAccounts<'info>,
}

impl<'info> TryFrom<Context<'info>> for Take<'info> {
    type Error = ProgramError;
    #[inline(always)]
    fn try_from(ctx: Context<'info>) -> Result<Self, Self::Error> {
        let accounts = TakeAccounts::try_from(ctx.accounts)?;
        Ok(Self { accounts })
    }
}

impl<'info> Take<'info> {
    pub fn execute(&self) -> ProgramResult {
        sol_log("Take invoked");

        // Deserialize escrow account
        let (amount_out, bump) = {
            let escrow_account_ref = self.accounts.escrow.try_borrow_data()?;
            let escrow_account: &EscrowAccount = escrow_account_ref.as_ref().try_into()?;

            // Account checks
            if self.accounts.mint_a.key().ne(&escrow_account.mint_a)
                || self.accounts.mint_b.key().ne(&escrow_account.mint_b)
                || self.accounts.maker.key().ne(&escrow_account.maker)
            {
                return Err(ProgramError::InvalidAccountData);
            }
            (escrow_account.amount_out, escrow_account.bump)
        };

        let seeds = [
            Seed::from(ESCROW_SEED),
            Seed::from(self.accounts.maker.key().as_ref()),
            Seed::from(self.accounts.mint_a.key().as_ref()),
            Seed::from(self.accounts.mint_b.key().as_ref()),
            Seed::from(&bump),
        ];
        let signers: [Signer; 1] = [Signer::from(&seeds)];

        // Create Taker token A account
        CreateIdempotent {
            funding_account: self.accounts.taker,
            account: self.accounts.taker_token_a,
            wallet: self.accounts.taker,
            mint: self.accounts.mint_a,
            system_program: self.accounts.system_program,
            token_program: self.accounts.token_program,
        }
        .invoke()?;

        // Create Maker token B account
        CreateIdempotent {
            funding_account: self.accounts.taker,
            account: self.accounts.maker_token_b,
            wallet: self.accounts.maker,
            mint: self.accounts.mint_b,
            system_program: self.accounts.system_program,
            token_program: self.accounts.token_program,
        }
        .invoke()?;

        // Transfer from taker to maker
        Transfer {
            from: self.accounts.taker_token_b,
            to: self.accounts.maker_token_b,
            authority: self.accounts.taker,
            amount: amount_out,
        }
        .invoke()?;

        // Deserialize vault account
        let amount = TokenAccount::from_account_info(self.accounts.vault)?.amount();

        // Transfer from vault to taker
        Transfer {
            from: self.accounts.vault,
            to: self.accounts.taker_token_a,
            authority: self.accounts.escrow,
            amount,
        }
        .invoke_signed(&signers)?;

        CloseAccount {
            account: self.accounts.vault,
            destination: self.accounts.taker,
            authority: self.accounts.escrow,
        }
        .invoke_signed(&signers)?;

        *self.accounts.taker.try_borrow_mut_lamports()? += self.accounts.escrow.lamports();
        self.accounts.escrow.close()
    }
}
