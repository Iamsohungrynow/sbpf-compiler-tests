use solana_pubkey::Pubkey;

#[allow(dead_code)]
pub mod account;
#[allow(dead_code)]
pub mod mint;
#[allow(dead_code)]
pub mod mollusk;

pub const TOKEN_PROGRAM_ID: Pubkey = Pubkey::new_from_array(pinocchio_token_interface::program::ID);

#[allow(dead_code)]
pub fn program_test() -> solana_program_test::ProgramTest {
    use std::sync::OnceLock;
    static PROGRAM_PATH: OnceLock<String> = OnceLock::new();
    let path = PROGRAM_PATH.get_or_init(harness::program_elf);
    let mut program = solana_program_test::ProgramTest::default();
    program.prefer_bpf(true);
    program.add_program(path, TOKEN_PROGRAM_ID, None);
    program
}
