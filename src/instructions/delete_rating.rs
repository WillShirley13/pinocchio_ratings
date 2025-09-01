use crate::{RatingAccount, RatingState, SignerAccount, SystemProgramAccount};
use pinocchio::msg;
use pinocchio::{
    account_info::{AccountInfo, Ref},
    instruction::{Seed, Signer},
    program_error::ProgramError,
    ProgramResult,
};

pub struct DeleteRatingAccounts<'a> {
    pub authority: &'a AccountInfo,
    pub rating: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
}

impl<'a> TryFrom<&'a [AccountInfo]> for DeleteRatingAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [authority, rating, system_program] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        Ok(Self {
            authority,
            rating,
            system_program,
        })
    }
}

pub struct DeleteRating<'a> {
    accounts: DeleteRatingAccounts<'a>,
}

impl<'a> TryFrom<&'a [AccountInfo]> for DeleteRating<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: DeleteRatingAccounts::try_from(accounts)?,
        })
    }
}

impl<'a> DeleteRating<'a> {
    pub const DISCRIMINATOR: u8 = 2;

    pub fn process(&mut self) -> ProgramResult {
        SignerAccount::check_is_signer(self.accounts.authority)?;
        msg!("Checked if authority is signer");
        SystemProgramAccount::check_is_system_program(self.accounts.system_program)?;
        msg!("Checked system program");

        let rating_data: Ref<'_, RatingState> = RatingState::load(self.accounts.rating)?;
        msg!("Loaded rating data");

        let movie_title_length = rating_data
            .movie_title
            .iter()
            .filter(|val| **val != 0u8)
            .count();

        RatingAccount::check_is_valid_rating(
            self.accounts.rating,
            self.accounts.authority,
            &rating_data.movie_title[..movie_title_length],
        )?;
        msg!("Validated rating account");

        let bump_slice: [u8; 1] = [rating_data.bump];

        let movie_title_seed: Vec<u8> = rating_data.movie_title[..movie_title_length].to_vec();
        drop(rating_data);

        let rating_lamports: u64 = self.accounts.rating.lamports();

        // Direct lamport manipulation
        *self.accounts.rating.try_borrow_mut_lamports()? -= rating_lamports;
        *self.accounts.authority.try_borrow_mut_lamports()? += rating_lamports;
        msg!("Transferred lamports back to authority");

        self.accounts.rating.close()?;
        msg!("Closed rating account");

        Ok(())
    }
}
