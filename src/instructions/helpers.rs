use crate::errors::RatingsErrors;
use pinocchio::{
    ProgramResult,
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::{Pubkey, find_program_address},
};

pub struct SignerAccount;
impl SignerAccount {
    pub fn check_is_signer(account: &AccountInfo) -> ProgramResult {
        if !account.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(())
    }
}

pub struct AdminAccount;
impl AdminAccount {
    pub fn check_is_valid_admin(admin_account: &AccountInfo) -> Result<u8, ProgramError> {
        let (true_admin_key, bump) = find_program_address(&[b"ratings_admin"], &crate::ID);
        if admin_account.key() != &true_admin_key {
            return Err(RatingsErrors::InvalidAdminAccount.into());
        }

        Ok(bump)
    }

    pub fn check_is_empty(account: &AccountInfo) -> Result<(), ProgramError> {
        if account.data_is_empty() {
            Ok(())
        } else {
            Err(RatingsErrors::ExpectedEmptyAccount.into())
        }
    }
}

pub struct RatingAccount;
impl RatingAccount {
    pub fn check_is_valid_rating(
        rating_account: &AccountInfo,
        user: &AccountInfo,
        movie_title: &str,
    ) -> Result<u8, ProgramError> {
        let (true_rating_key, bump) =
            find_program_address(&[user.key().as_ref(), movie_title.as_bytes()], &crate::ID);

        if rating_account.key() != &true_rating_key {
            return Err(RatingsErrors::InvalidRatingAccount.into());
        }

        Ok(bump)
    }

    pub fn check_is_empty(account: &AccountInfo) -> Result<(), ProgramError> {
        if account.data_is_empty() {
            Ok(())
        } else {
            Err(RatingsErrors::ExpectedEmptyAccount.into())
        }
    }

    pub fn convert_bytes_to_string(bytes: &[u8]) -> String {
        String::from_utf8(bytes.to_vec()).unwrap()
    }
}

pub struct SystemProgramAccount;
impl SystemProgramAccount {
    pub fn check_is_system_program(account: &AccountInfo) -> Result<(), ProgramError> {
        if !account.is_owned_by(&pinocchio_system::ID) {
            return Err(RatingsErrors::InvalidOwner.into());
        }

        Ok(())
    }
}

// Token 2022 program ID - manually defined since pinocchio doesn't expose it
const TOKEN_2022_PROGRAM_ID: Pubkey = [
    6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180, 133, 237,
    95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169,
];

pub struct TokenProgramAccount;
impl TokenProgramAccount {
    pub fn check_is_token_program(account: &AccountInfo) -> Result<(), ProgramError> {
        if account.key() != &pinocchio_token::ID && account.key() != &TOKEN_2022_PROGRAM_ID {
            return Err(RatingsErrors::InvalidOwner.into());
        }

        Ok(())
    }
}

pub struct AssociateTokenProgram;
impl AssociateTokenProgram {
    pub fn check_is_associate_token_program(account: &AccountInfo) -> Result<(), ProgramError> {
        if account.key() != &pinocchio_associated_token_account::ID {
            return Err(RatingsErrors::InvalidOwner.into());
        }
        Ok(())
    }
}

pub struct AssociatedTokenAccount;
impl AssociatedTokenAccount {
    pub fn check_is_valid_ata(
        ata: &AccountInfo,
        owner: &AccountInfo,
        mint: &AccountInfo,
    ) -> ProgramResult {
        let (true_ata, _) = pinocchio::pubkey::find_program_address(
            &[owner.key(), pinocchio_token::ID.as_ref(), mint.key()],
            &pinocchio_associated_token_account::ID,
        );

        if Pubkey::from(true_ata) != *ata.key() {
            return Err(RatingsErrors::InvalidAssociatedTokenAccount.into());
        }

        Ok(())
    }
}

pub struct SystemAccount;
impl SystemAccount {
    pub fn check_is_system_account(account: &AccountInfo) -> Result<(), ProgramError> {
        if !account.is_owned_by(&pinocchio_system::ID) {
            return Err(RatingsErrors::InvalidOwner.into());
        }

        Ok(())
    }
}

pub struct MintAccount;

impl MintAccount {
    pub fn check_is_mint(account: &AccountInfo, true_mint: &Pubkey) -> Result<(), ProgramError> {
        if !account.is_owned_by(&pinocchio_token::ID) {
            return Err(RatingsErrors::InvalidOwner.into());
        }

        if account.data_len() != pinocchio_token::state::Mint::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        if account.key() != true_mint {
            return Err(RatingsErrors::InvalidMintAccount.into());
        }

        Ok(())
    }
}

pub struct TokenAccount;

impl TokenAccount {
    pub fn check(account: &AccountInfo) -> Result<(), ProgramError> {
        if !account.is_owned_by(&pinocchio_token::ID) {
            return Err(RatingsErrors::InvalidOwner.into());
        }

        if account
            .data_len()
            .ne(&pinocchio_token::state::TokenAccount::LEN)
        {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }

    pub fn get_amount(account: &AccountInfo) -> u64 {
        let token_account: pinocchio::account_info::Ref<'_, pinocchio_token::state::TokenAccount> =
            pinocchio_token::state::TokenAccount::from_account_info(account).unwrap();
        token_account.amount()
    }
}
