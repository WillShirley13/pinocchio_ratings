use crate::{RatingAccount, RatingState, SignerAccount, SystemProgramAccount};
use pinocchio::{
    account_info::{AccountInfo, Ref},
    instruction::{Seed, Signer},
    program_error::ProgramError,
    ProgramResult,
};
use pinocchio_system::instructions::Transfer;

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
        SystemProgramAccount::check_is_system_program(self.accounts.system_program)?;

        let rating_data: Ref<'_, RatingState> = RatingState::load(self.accounts.rating)?;

        RatingAccount::check_is_valid_rating(
            self.accounts.rating,
            self.accounts.authority,
            &rating_data.movie_title,
        )?;

        let rating_lamports: u64 = self.accounts.rating.lamports();

        let seeds: [Seed<'_>; 2] = [
            Seed::from(self.accounts.authority.key().as_ref()),
            Seed::from(rating_data.movie_title.as_ref()),
        ]; // POTENTIALLY TROUBLESOME

        Transfer {
            from: self.accounts.rating,
            to: self.accounts.authority,
            lamports: rating_lamports,
        }
        .invoke_signed(&[Signer::from(&seeds)])?;

        self.accounts.rating.close()?;

        Ok(())
    }
}
