use pinocchio::{
    account_info::{AccountInfo, Ref, RefMut},
    program_error::ProgramError,
    pubkey::Pubkey,
};
use std::mem::size_of;

use crate::errors::RatingsErrors;

#[repr(C)]
pub struct AdminState {
    pub authority: Pubkey,
    pub token_mint: Pubkey,
    pub reward_amount: u64,
    pub bump: u8,
}

impl AsRef<[u8]> for AdminState {
    fn as_ref(&self) -> &[u8] {
        // SAFETY: repr(C) + POD ⇒ byte-compatible
        unsafe { core::slice::from_raw_parts(self as *const Self as *const u8, Self::LEN) }
    }
}

impl AdminState {
    pub const LEN: usize = 32 + 32 + 8 + 8; // 8 bytes for bump to ensure memory alignment

    #[inline(always)]
    pub fn load(account: &AccountInfo) -> Result<Ref<Self>, ProgramError> {
        if account.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        if account.owner() != &crate::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }

        Ok(Ref::map(account.try_borrow_data()?, |data| unsafe {
            &*(data.as_ptr() as *const AdminState)
        }))
    }

    #[inline(always)]
    pub fn load_mut(account: &AccountInfo) -> Result<RefMut<AdminState>, ProgramError> {
        if account.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        if account.owner() != &crate::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }
        Ok(RefMut::map(account.try_borrow_mut_data()?, |data| unsafe {
            &mut *(data.as_mut_ptr() as *mut AdminState)
        }))
    }

    #[inline(always)]
    pub fn set_authority(&mut self, authority: Pubkey) -> Result<(), ProgramError> {
        self.authority = authority;
        Ok(())
    }

    #[inline(always)]
    pub fn set_token_mint(&mut self, token_mint: Pubkey) -> Result<(), ProgramError> {
        self.token_mint = token_mint;
        Ok(())
    }

    #[inline(always)]
    pub fn set_reward_amount(&mut self, reward_amount: u64) -> Result<(), ProgramError> {
        self.reward_amount = reward_amount;
        Ok(())
    }

    #[inline(always)]
    pub fn set_bump(&mut self, bump: u8) -> Result<(), ProgramError> {
        self.bump = bump;
        Ok(())
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RatingState {
    pub movie_title: [u8; 32], // Movie title (max 32 chars)
    pub rating: u8,            // Rating 1-10
    pub owner: Pubkey,         // User who created the rating
    pub timestamp: i64,        // Unix timestamp of creation
    pub bump: u8,
}

impl AsRef<[u8]> for RatingState {
    fn as_ref(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const Self as *const u8, Self::LEN) }
    }
}

impl RatingState {
    pub const LEN: usize = 32 + 8 + 32 + 8 + 8; // 8 bytes for rating and bump to ensure memory alignment

    #[inline(always)]
    pub fn load(account: &AccountInfo) -> Result<Ref<Self>, ProgramError> {
        if account.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        if account.owner() != &crate::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }

        Ok(Ref::map(account.try_borrow_data()?, |data: &[u8]| unsafe {
            &*(data.as_ptr() as *const RatingState)
        }))
    }

    pub fn load_mut(account: &AccountInfo) -> Result<RefMut<Self>, ProgramError> {
        if account.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        if account.owner() != &crate::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }

        Ok(RefMut::map(account.try_borrow_mut_data()?, |data| unsafe {
            &mut *(data.as_mut_ptr() as *mut RatingState)
        }))
    }

    pub fn set_movie_title(&mut self, movie_title: String) -> Result<(), ProgramError> {
        if movie_title.len() > 32 {
            return Err(RatingsErrors::MovieTitleTooLong.into());
        }

        let mut movie_title_array = [0u8; 32];
        let movie_title_bytes = movie_title.as_bytes();
        movie_title_array[..movie_title_bytes.len()].copy_from_slice(movie_title_bytes);
        self.movie_title = movie_title_array;

        Ok(())
    }

    pub fn set_rating(&mut self, rating: u8) -> Result<(), ProgramError> {
        if !(1..=10).contains(&rating) {
            return Err(RatingsErrors::InvalidRatingValue.into());
        }

        self.rating = rating;

        Ok(())
    }

    #[inline(always)]
    pub fn set_owner(&mut self, owner: Pubkey) -> Result<(), ProgramError> {
        self.owner = owner;
        Ok(())
    }

    #[inline(always)]
    pub fn set_timestamp(&mut self, timestamp: i64) -> Result<(), ProgramError> {
        self.timestamp = timestamp;
        Ok(())
    }

    #[inline(always)]
    pub fn set_inner(
        movie_title: String, // Movie title (max 32 chars)
        rating: u8,          // Rating 1-10
        owner: Pubkey,       // User who created the rating
        timestamp: i64,      // Unix timestamp of creation
        bump: u8,
    ) -> Result<Self, ProgramError> {
        if movie_title.len() > 32 {
            return Err(RatingsErrors::MovieTitleTooLong.into());
        }

        let mut movie_title_array: [u8; 32] = [0u8; 32];
        let movie_title_bytes: &[u8] = movie_title.as_bytes();
        movie_title_array[..movie_title_bytes.len()].copy_from_slice(movie_title_bytes);

        Ok(Self {
            movie_title: movie_title_array,
            rating,
            owner,
            timestamp,
            bump,
        })
    }
}
