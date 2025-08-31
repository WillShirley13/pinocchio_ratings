use pinocchio::{account_info::AccountInfo, entrypoint, pubkey::Pubkey, ProgramResult};

entrypoint!(process_instruction);

pub mod instructions;
pub use instructions::*;

pub mod errors;
pub use errors::*;

pub mod state;
pub use state::*;

pub const ID: Pubkey = [
    0x8c, 0xff, 0xc2, 0x21, 0x92, 0x2d, 0x48, 0x74, 0x7a, 0x82, 0xe5, 0xc5, 0x08, 0x70, 0x45, 0x90,
    0xda, 0x82, 0x11, 0x57, 0x89, 0x65, 0x1f, 0x51, 0x63, 0x62, 0x3d, 0x54, 0x72, 0x4d, 0x64, 0xd4,
];

fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    match data.split_first() {
        Some((&InitAdmin::DISCRIMINATOR, data)) => InitAdmin::try_from((accounts, data))?.process(),
        Some((&InitRating::DISCRIMINATOR, data)) => {
            InitRating::try_from((accounts, data))?.process()
        }
        Some((&DeleteRating::DISCRIMINATOR, _)) => DeleteRating::try_from(accounts)?.process(),
        _ => Err(RatingsErrors::InvalidInstruction.into()),
    }
}
