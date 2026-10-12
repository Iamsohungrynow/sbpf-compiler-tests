use pinocchio::account_info::AccountInfo;

pub struct Context<'info> {
    pub accounts: &'info [AccountInfo],
    pub instruction_data: &'info [u8],
}

impl<'info> From<(&'info [AccountInfo], &'info [u8])> for Context<'info> {
    fn from(value: (&'info [AccountInfo], &'info [u8])) -> Self {
        Context {
            accounts: value.0,
            instruction_data: value.1,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EscrowAccount {
    pub maker: [u8; 32],
    pub mint_a: [u8; 32],
    pub mint_b: [u8; 32],
    pub amount_out: u64,
    pub bump: [u8; 1],
}

impl<'info> TryFrom<&'info mut [u8]> for &'info mut EscrowAccount {
    type Error = pinocchio::program_error::ProgramError;

    fn try_from(data: &'info mut [u8]) -> Result<Self, Self::Error> {
        if data.len() != core::mem::size_of::<EscrowAccount>() {
            return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut EscrowAccount) })
    }
}

impl<'info> TryFrom<&'info [u8]> for &'info EscrowAccount {
    type Error = pinocchio::program_error::ProgramError;

    fn try_from(data: &'info [u8]) -> Result<Self, Self::Error> {
        if data.len() != core::mem::size_of::<EscrowAccount>() {
            return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &*(data.as_ptr() as *mut EscrowAccount) })
    }
}
