use crate::{
    AdminAccount, AdminState, AssociateTokenProgram, AssociatedTokenAccount, MintAccount,
    RatingAccount, RatingState, SystemProgramAccount, TokenProgramAccount,
};
use pinocchio::instruction::{Seed, Signer};
use pinocchio::program_error::ProgramError;
use pinocchio::{
    account_info::{AccountInfo, Ref, RefMut},
    sysvars::{clock, rent::Rent, Sysvar},
};
use pinocchio::{msg, ProgramResult};
use pinocchio_associated_token_account::instructions::Create;
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::instructions::TransferChecked;
use pinocchio_token::state::Mint;
use pinocchio_token::state::TokenAccount as PinoTokenAccount;

pub struct InitRatingAccounts<'a> {
    pub authority: &'a AccountInfo,
    pub rating: &'a AccountInfo,
    pub authority_ata: &'a AccountInfo,
    pub admin: &'a AccountInfo,
    pub admin_ata: &'a AccountInfo,
    pub ratings_mint: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub token_program: &'a AccountInfo,
    pub associated_token_program: &'a AccountInfo,
    pub rating_bump: u8,
}

impl<'a> TryFrom<&'a [AccountInfo]> for InitRatingAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        msg!("Configuring InitRatingAccounts accounts");
        let [authority, rating, authority_ata, admin, admin_ata, ratings_mint, system_program, token_program, associated_token_program] =
            accounts
        else {
            return Err(ProgramError::InvalidArgument);
        };

        Ok(Self {
            authority,
            rating,
            authority_ata,
            admin,
            admin_ata,
            ratings_mint,
            system_program,
            token_program,
            associated_token_program,
            rating_bump: 0, // Placeholder, will be set in process
        })
    }
}

pub struct InitRatingPayload {
    pub movie_title: String,
    pub rating: u8,
}

impl TryFrom<&[u8]> for InitRatingPayload {
    type Error = ProgramError;

    fn try_from(data: &[u8]) -> Result<Self, Self::Error> {
        msg!("Parsing InitRatingPayload");
        if data.len() < 2 {
            // at least 1 byte title + 1 byte rating
            return Err(ProgramError::InvalidInstructionData);
        }

        let rating = data[data.len() - 1]; // last byte is rating
        let title_bytes = &data[..data.len() - 1]; // everything else is title
        let movie_title = String::from_utf8(title_bytes.to_vec())
            .map_err(|_| ProgramError::InvalidInstructionData)?;

        // validation...

        Ok(Self {
            movie_title,
            rating,
        })
    }
}

pub struct InitRating<'a> {
    pub accounts: InitRatingAccounts<'a>,
    pub payload: InitRatingPayload,
}

impl<'a> TryFrom<(&'a [AccountInfo], &[u8])> for InitRating<'a> {
    type Error = ProgramError;

    fn try_from(input: (&'a [AccountInfo], &[u8])) -> Result<Self, Self::Error> {
        let (accounts_slice, data) = input;
        let accounts: InitRatingAccounts<'_> = InitRatingAccounts::try_from(accounts_slice)?;
        let payload: InitRatingPayload = InitRatingPayload::try_from(data)?; // Assuming you have this impl
        Ok(Self { accounts, payload })
    }
}

impl<'a> InitRating<'a> {
    pub const DISCRIMINATOR: u8 = 1;

    pub fn process(&mut self) -> ProgramResult {
        let accounts: &mut InitRatingAccounts<'_> = &mut self.accounts;
        let payload: &InitRatingPayload = &self.payload;

        // Perform validations here
        AdminAccount::check_is_valid_admin(accounts.admin)?;
        msg!("Admin account validated");
        accounts.rating_bump = RatingAccount::check_is_valid_rating(
            accounts.rating,
            accounts.authority,
            payload.movie_title.as_bytes(),
        )?;
        msg!("Rating account validated");
        RatingAccount::check_is_empty(accounts.rating)?;
        msg!("Rating account is empty");
        SystemProgramAccount::check_is_system_program(accounts.system_program)?;
        TokenProgramAccount::check_is_token_program(accounts.token_program)?;
        msg!("Token program validated");
        AssociateTokenProgram::check_is_associate_token_program(accounts.associated_token_program)?;
        msg!("Associated token program validated");
        AssociatedTokenAccount::check_is_valid_ata(
            accounts.authority_ata,
            accounts.authority,
            accounts.ratings_mint,
        )?;
        msg!("Authority ATA account validated");
        AssociatedTokenAccount::check_is_valid_ata(
            accounts.admin_ata,
            accounts.admin,
            accounts.ratings_mint,
        )?;
        msg!("Admin ATA account validated");

        // Load admin state
        let admin_data: Ref<'_, AdminState> = AdminState::load(accounts.admin)?;
        msg!("Admin state loaded");
        MintAccount::check_is_mint(accounts.ratings_mint, &admin_data.token_mint)?;
        msg!("Mint account validated");

        // Set Rating seeds
        let bump_slice: [u8; 1] = [accounts.rating_bump];
        let rating_seeds: [Seed<'_>; 3] = [
            Seed::from(accounts.authority.key().as_ref()),
            Seed::from(payload.movie_title.as_bytes()),
            Seed::from(&bump_slice),
        ];
        let rating_signer: [Signer<'_, '_>; 1] = [Signer::from(&rating_seeds)];

        let rent: Rent = Rent::get()?;

        // Init Rating pda
        CreateAccount {
            from: accounts.authority,
            to: accounts.rating,
            lamports: rent.minimum_balance(RatingState::LEN),
            space: RatingState::LEN as u64,
            owner: &crate::ID,
        }
        .invoke_signed(&rating_signer)?;
        msg!("Rating account created");

        // Build and serilaise Rating data
        let rating_state: RatingState = RatingState::set_inner(
            payload.movie_title.to_owned(),
            payload.rating,
            *accounts.authority.key(),
            clock::Clock::get()?.unix_timestamp,
            accounts.rating_bump,
        )?;

        let mut rating_data: RefMut<'_, [u8]> = accounts.rating.try_borrow_mut_data()?;
        rating_data[..RatingState::LEN].copy_from_slice(rating_state.as_ref());
        msg!("Rating data serialized");

        // Init Authority ATA if it doesn't exist
        if accounts.authority_ata.data_len() != PinoTokenAccount::LEN {
            Create {
                funding_account: accounts.authority,
                account: accounts.authority_ata,
                wallet: accounts.authority,
                mint: accounts.ratings_mint,
                system_program: accounts.system_program,
                token_program: accounts.token_program,
            }
            .invoke()?;
            msg!("Authority ATA created");
        }

        // Transfer tokens from admin to authority
        let mint_data: Ref<'_, Mint> = Mint::from_account_info(accounts.ratings_mint)?;
        TransferChecked {
            from: accounts.admin_ata,
            mint: accounts.ratings_mint,
            to: accounts.authority_ata,
            authority: accounts.admin,
            amount: admin_data.reward_amount,
            decimals: mint_data.decimals(),
        }
        .invoke_signed(&[Signer::from(&[
            Seed::from(b"ratings_admin"),
            Seed::from(&[admin_data.bump]),
        ])])?;
        msg!("Tokens transferred from admin to authority");
        Ok(())
    }
}
