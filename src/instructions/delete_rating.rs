use std::str::from_utf8;

use pinocchio::{
    ProgramResult,
    account_info::{AccountInfo, Ref},
    program_error::ProgramError,
};

use crate::{RatingAccount, RatingState, SignerAccount, SystemProgramAccount};

pub struct DeleteRatingAccounts<'a> {
    pub authority: &'a AccountInfo,
    pub rating: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
}

impl<'a> TryFrom<&'a [AccountInfo]> for DeleteRatingAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [authority, rating, system_program, _] = accounts else {
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
    pub fn process(&mut self) -> ProgramResult {
        SignerAccount::check_is_signer(self.accounts.authority)?;
        SystemProgramAccount::check_is_system_program(self.accounts.system_program)?;

        let rating_data: Ref<'_, RatingState> = RatingState::load(self.accounts.rating)?;

        RatingAccount::check_is_valid_rating(
            self.accounts.rating,
            self.accounts.authority,
            str::from_utf8(&rating_data.movie_title)
                .map_err(|_| ProgramError::InvalidAccountData)?,
        )?;

        Ok(())
    }
}
