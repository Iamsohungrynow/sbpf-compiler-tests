use pinocchio::{
    ProgramResult,
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    log::sol_log,
    program_error::ProgramError,
};
use pinocchio_token::{
    instructions::{CloseAccount, Transfer},
    state::TokenAccount,
};

use crate::{
    constants::ESCROW_SEED,
    state::{Context, EscrowAccount},
};

#[derive(Debug)]
pub struct RefundAccounts<'info> {
    maker: &'info AccountInfo,
    maker_token_a: &'info AccountInfo,
    escrow: &'info AccountInfo,
    vault: &'info AccountInfo,
    _token_program: &'info AccountInfo,
    _system_program: &'info AccountInfo,
}

impl<'info> TryFrom<&'info [AccountInfo]> for RefundAccounts<'info> {
    type Error = ProgramError;

    fn try_from(accounts: &'info [AccountInfo]) -> Result<Self, Self::Error> {
        let [
            maker,
            maker_token_a,
            escrow,
            vault,
            _token_program,
            _system_program,
        ] = accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // Account Checks
        if !maker.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            maker,
            maker_token_a,
            escrow,
            vault,
            _token_program,
            _system_program,
        })
    }
}

pub struct Refund<'info> {
    accounts: RefundAccounts<'info>,
}

impl<'info> TryFrom<Context<'info>> for Refund<'info> {
    type Error = ProgramError;
    #[inline(always)]
    fn try_from(ctx: Context<'info>) -> Result<Self, Self::Error> {
        let accounts = RefundAccounts::try_from(ctx.accounts)?;

        Ok(Self { accounts })
    }
}

impl<'info> Refund<'info> {
    #[inline(always)]
    pub fn execute(&self) -> ProgramResult {
        sol_log("Refund invoked");

        let (mint_a, mint_b, bump) = {
            let escrow_account_ref = self.accounts.escrow.try_borrow_data()?;
            let escrow_account: &EscrowAccount = escrow_account_ref.as_ref().try_into()?;
            if self.accounts.maker.key().ne(&escrow_account.maker) {
                return Err(ProgramError::InvalidAccountData);
            }
            (
                escrow_account.mint_a,
                escrow_account.mint_b,
                escrow_account.bump,
            )
        };

        let seeds = [
            Seed::from(ESCROW_SEED),
            Seed::from(self.accounts.maker.key().as_ref()),
            Seed::from(mint_a.as_ref()),
            Seed::from(mint_b.as_ref()),
            Seed::from(&bump),
        ];
        let signers: [Signer; 1] = [Signer::from(&seeds)];

        let amount = TokenAccount::from_account_info(self.accounts.vault)?.amount();

        // Deposit initial capital to vault
        Transfer {
            from: self.accounts.vault,
            to: self.accounts.maker_token_a,
            authority: self.accounts.escrow,
            amount,
        }
        .invoke_signed(&signers)?;

        CloseAccount {
            account: self.accounts.vault,
            destination: self.accounts.maker,
            authority: self.accounts.escrow,
        }
        .invoke_signed(&signers)?;

        *self.accounts.maker.try_borrow_mut_lamports()? += self.accounts.escrow.lamports();
        self.accounts.escrow.close()
    }
}
