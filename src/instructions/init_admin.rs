use crate::{AdminState, helpers::*};
use pinocchio::{
    ProgramResult,
    account_info::{AccountInfo, RefMut},
    instruction::{Seed, Signer},
    program_error::ProgramError,
    sysvars::{Sysvar, rent::Rent},
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::{
    instructions::{InitializeAccount3, InitializeMint2, MintTo},
    state::Mint,
};
pub struct InitAdminAccounts<'a> {
    authority: &'a AccountInfo,
    admin: &'a AccountInfo,
    ratings_mint: &'a AccountInfo,
    admin_ata: &'a AccountInfo,
    system_program: &'a AccountInfo,
    token_program: &'a AccountInfo,
    associated_token_program: &'a AccountInfo,
    bump: u8,
}

impl<'a> TryFrom<&'a [AccountInfo]> for InitAdminAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [
            authority,
            admin,
            ratings_mint,
            admin_ata,
            system_program,
            token_program,
            associated_token_program,
        ] = accounts
        else {
            return Err(ProgramError::InvalidArgument);
        };

        Ok(Self {
            authority,
            admin,
            ratings_mint,
            admin_ata,
            system_program,
            token_program,
            associated_token_program,
            bump: 0, // Placeholder, will be set in process
        })
    }
}

pub struct InitAdminPayload {
    pub reward_amount: u64,
}

impl TryFrom<&[u8]> for InitAdminPayload {
    type Error = ProgramError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        let payload: u64 = u64::from_le_bytes(
            data[..8]
                .try_into()
                .map_err(|_| ProgramError::InvalidAccountData)?,
        );
        Ok(Self {
            reward_amount: payload,
        })
    }
}

pub struct InitAdmin<'a> {
    pub accounts: InitAdminAccounts<'a>,
    pub payload: InitAdminPayload,
}

impl<'a> TryFrom<(&'a [AccountInfo], &[u8])> for InitAdmin<'a> {
    type Error = ProgramError;

    fn try_from(input: (&'a [AccountInfo], &[u8])) -> Result<Self, Self::Error> {
        let (accounts, data) = input;
        let accounts: InitAdminAccounts<'_> = InitAdminAccounts::try_from(accounts)?;
        let payload: InitAdminPayload = InitAdminPayload::try_from(data)?;
        Ok(Self { accounts, payload })
    }
}

impl<'a> InitAdmin<'a> {
    pub const DISCRIMINATOR: u8 = 0;

    pub fn process(self) -> ProgramResult {
        let mut accounts: InitAdminAccounts<'a> = self.accounts; // Make mutable if needed
        let payload: InitAdminPayload = self.payload;

        // Moved validations
        SignerAccount::check_is_signer(accounts.authority)?;
        accounts.bump = AdminAccount::check_is_valid_admin(accounts.admin)?;
        AdminAccount::check_is_empty(accounts.admin)?;
        SystemProgramAccount::check_is_system_program(accounts.system_program)?;
        TokenProgramAccount::check_is_token_program(accounts.token_program)?;
        AssociateTokenProgram::check_is_associate_token_program(accounts.associated_token_program)?;
        AssociatedTokenAccount::check_is_valid_ata(
            accounts.admin_ata,
            accounts.authority,
            accounts.ratings_mint,
        )?;

        let bump_slice: [u8; 1] = [accounts.bump];

        let seeds: [Seed<'_>; 2] = [
            Seed::from(b"ratings_admin"),
            Seed::from(bump_slice.as_ref()),
        ];
        let signer: [Signer<'_, '_>; 1] = [Signer::from(&seeds)];

        // Create admin account
        let admin_rent = Rent::get()?.minimum_balance(AdminState::LEN);

        CreateAccount {
            from: accounts.authority,
            to: accounts.admin,
            lamports: admin_rent,
            space: AdminState::LEN as u64,
            owner: &crate::ID,
        }
        .invoke_signed(&signer)?;

        // Create and Init Mint
        let mint_rent: u64 = Rent::get()?.minimum_balance(Mint::LEN);

        CreateAccount {
            from: accounts.authority,
            to: accounts.ratings_mint,
            lamports: mint_rent,
            space: Mint::LEN as u64,
            owner: &pinocchio_token::ID,
        }
        .invoke()?;

        InitializeMint2 {
            mint: accounts.ratings_mint,
            mint_authority: accounts.admin.key(),
            decimals: 9,
            freeze_authority: None,
        }
        .invoke()?;

        // Init Admin associated token account
        InitializeAccount3 {
            account: accounts.admin_ata,
            mint: accounts.ratings_mint,
            owner: accounts.admin.key(),
        }
        .invoke()?;

        // Mint (1000 * reward_amount) to admin associated token account
        MintTo {
            mint: accounts.ratings_mint,
            account: accounts.admin_ata,
            mint_authority: accounts.admin,
            amount: payload.reward_amount * 1000,
        }
        .invoke()?;

        // Write admin state to admin account
        let admin_state = {
            AdminState {
                authority: *accounts.authority.key(),
                token_mint: *accounts.ratings_mint.key(),
                reward_amount: payload.reward_amount,
                bump: accounts.bump,
            }
        };

        let mut admin_data: RefMut<'_, [u8]> = accounts.admin.try_borrow_mut_data()?;
        admin_data[..AdminState::LEN].copy_from_slice(admin_state.as_ref());

        Ok(())
    }
}
