use crate::helpers::TokenAccount;
use crate::{
    AdminAccount, AdminState, AssociateTokenProgram, AssociatedTokenAccount, MintAccount,
    RatingAccount, RatingState, RatingsErrors, SystemProgramAccount, TokenProgramAccount,
};
use pinocchio::ProgramResult;
use pinocchio::instruction::{Seed, Signer};
use pinocchio::program_error::ProgramError;
use pinocchio::{
    account_info::{AccountInfo, Ref, RefMut},
    sysvars::{Sysvar, clock, rent::Rent},
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::instructions::TransferChecked;
use pinocchio_token::state::Mint;
use pinocchio_token::{
    ID as TOKEN_PROGRAM_ID, instructions::InitializeAccount3,
    state::TokenAccount as PinoTokenAccount,
};

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
        let [
            authority,
            rating,
            authority_ata,
            admin,
            admin_ata,
            ratings_mint,
            system_program,
            token_program,
            associated_token_program,
            _,
        ] = accounts
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
        if data.len() < 5 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let mut offset = 0;
        let title_len = u32::from_le_bytes(
            data[offset..offset + 4]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?,
        );
        offset += 4;
        if data.len() < offset + title_len as usize + 1 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let movie_title = String::from_utf8(data[offset..offset + title_len as usize].to_vec())
            .map_err(|_| ProgramError::InvalidInstructionData)?;
        offset += title_len as usize;
        let rating = data[offset];

        if movie_title.len() > 32 {
            return Err(RatingsErrors::MovieTitleTooLong.into());
        }

        if rating < 1 || rating > 10 {
            return Err(RatingsErrors::InvalidRatingValue.into());
        }

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
    pub fn process(&mut self) -> ProgramResult {
        let accounts: &mut InitRatingAccounts<'_> = &mut self.accounts;
        let payload: &InitRatingPayload = &self.payload;

        // Perform validations here
        AdminAccount::check_is_valid_admin(accounts.admin)?;
        accounts.rating_bump = RatingAccount::check_is_valid_rating(
            accounts.rating,
            accounts.authority,
            &payload.movie_title,
        )?;
        RatingAccount::check_is_empty(accounts.rating)?;
        SystemProgramAccount::check_is_system_program(accounts.system_program)?;
        TokenProgramAccount::check_is_token_program(accounts.token_program)?;
        AssociateTokenProgram::check_is_associate_token_program(accounts.associated_token_program)?;
        TokenAccount::check(accounts.authority_ata)?;
        TokenAccount::check(accounts.admin_ata)?;
        AssociatedTokenAccount::check_is_valid_ata(
            accounts.authority_ata,
            accounts.authority,
            accounts.ratings_mint,
        )?;
        AssociatedTokenAccount::check_is_valid_ata(
            accounts.admin_ata,
            accounts.admin,
            accounts.ratings_mint,
        )?;

        let admin_data: Ref<'_, AdminState> = AdminState::load(accounts.admin)?;

        MintAccount::check_is_mint(accounts.ratings_mint, &admin_data.token_mint)?;

        // TODO: Add the rest of your init logic here, e.g., create account, etc.

        let rent: Rent = Rent::get()?;

        // Init Rating pda
        CreateAccount {
            from: accounts.authority,
            to: accounts.rating,
            lamports: rent.minimum_balance(RatingState::LEN),
            space: RatingState::LEN as u64,
            owner: &crate::ID,
        }
        .invoke()?;

        // Build and serilaise Rating data
        let rating_state: RatingState = RatingState::set_inner(
            payload.movie_title.to_owned(),
            payload.rating,
            *accounts.authority.key(),
            clock::Clock::get()?.unix_timestamp as i64,
            accounts.rating_bump,
        )?;

        let mut rating_data: RefMut<'_, [u8]> = accounts.rating.try_borrow_mut_data()?;
        rating_data[..RatingState::LEN].copy_from_slice(rating_state.as_ref());

        // Init Authority ATA if it doesn't exist
        if accounts.authority_ata.data_len() != PinoTokenAccount::LEN {
            InitializeAccount3 {
                account: accounts.authority_ata,
                mint: accounts.ratings_mint,
                owner: &TOKEN_PROGRAM_ID,
            }
            .invoke()?;
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
            Seed::from(&[accounts.rating_bump]),
        ])])?;

        Ok(())
    }
}
