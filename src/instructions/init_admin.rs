use crate::{helpers::*, AdminState};
use pinocchio::{
    account_info::{AccountInfo, RefMut},
    instruction::{Seed, Signer},
    msg,
    program_error::ProgramError,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_associated_token_account::instructions::Create;
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::{
    instructions::{InitializeMint2, MintTo},
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
        let [authority, admin, ratings_mint, admin_ata, system_program, token_program, associated_token_program] =
            accounts
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
        msg!("Instruction: InitAdmin");
        let mut accounts: InitAdminAccounts<'a> = self.accounts; // Make mutable if needed
        let payload: InitAdminPayload = self.payload;

        // Moved validations
        SignerAccount::check_is_signer(accounts.authority)?;
        msg!("Authority account validated");
        accounts.bump = AdminAccount::check_is_valid_admin(accounts.admin)?;
        msg!("Admin account validated");
        AdminAccount::check_is_empty(accounts.admin)?;
        msg!("Admin account is empty");
        SystemProgramAccount::check_is_system_program(accounts.system_program)?;
        msg!("System program validated");
        TokenProgramAccount::check_is_token_program(accounts.token_program)?;
        msg!("Token program validated");
        AssociateTokenProgram::check_is_associate_token_program(accounts.associated_token_program)?;
        msg!("Associated token program validated");
        AssociatedTokenAccount::check_is_valid_ata(
            accounts.admin_ata,
            accounts.admin,
            accounts.ratings_mint,
        )?;
        msg!("Admin ATA account validated");
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
        msg!("Admin account created");
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
        msg!("Mint account created");
        InitializeMint2 {
            mint: accounts.ratings_mint,
            mint_authority: accounts.admin.key(),
            decimals: 9,
            freeze_authority: None,
        }
        .invoke()?;
        msg!("Mint initialized");

        // Init Admin associated token account
        // Add before InitializeAccount3:

        Create {
            funding_account: accounts.authority,
            account: accounts.admin_ata,
            wallet: accounts.admin,
            mint: accounts.ratings_mint,
            system_program: accounts.system_program,
            token_program: accounts.token_program,
        }
        .invoke()?;
        msg!("Admin ATA account created and initialized");

        // Mint (1000 * reward_amount) to admin associated token account
        MintTo {
            mint: accounts.ratings_mint,
            account: accounts.admin_ata,
            mint_authority: accounts.admin,
            amount: payload.reward_amount * 1000,
        }
        .invoke_signed(&signer)?;
        msg!("Minted to admin ATA account");
        // Write admin state to admin account
        let admin_state = {
            AdminState {
                authority: *accounts.authority.key(),
                token_mint: *accounts.ratings_mint.key(),
                reward_amount: payload.reward_amount,
                bump: accounts.bump,
            }
        };
        msg!("Admin state created");
        let mut admin_data: RefMut<'_, [u8]> = accounts.admin.try_borrow_mut_data()?;
        admin_data[..AdminState::LEN].copy_from_slice(admin_state.as_ref());
        msg!("Admin state written to admin account");
        Ok(())
    }
}
