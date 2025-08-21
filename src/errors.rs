use pinocchio::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RatingsErrors {
    #[error("Invalid Admin account")]
    InvalidAdminAccount,
    #[error("Invalid Rating account")]
    InvalidRatingAccount,
    #[error("Invalid associate token account")]
    InvalidAssociatedTokenAccount,
    #[error("Invalid owner")]
    InvalidOwner,
    #[error("Movie title too long")]
    MovieTitleTooLong,
    #[error("Invalid rating value")]
    InvalidRatingValue,
    #[error("Expected empty account")]
    ExpectedEmptyAccount,
    #[error("Invalid mint account")]
    InvalidMintAccount,
}

impl From<RatingsErrors> for ProgramError {
    fn from(value: RatingsErrors) -> Self {
        ProgramError::Custom(value as u32)
    }
}
