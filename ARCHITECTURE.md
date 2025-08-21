# Movie Rating Solana Program Architecture

## Overview

A Solana program built with pinocchio that allows users to submit movie ratings and receive token rewards. The program maintains an admin-controlled token mint and stores user ratings as PDAs.

## Account Types

### 1. Admin Account (PDA)

- **Purpose**: Single admin account that controls the token mint and program operations
- **Seeds**: `["admin"]`
- **Data Structure**:

  ```rust
  pub struct Admin {
      pub authority: Pubkey,        // Admin wallet authority
      pub token_mint: Pubkey,       // Token mint address
      pub reward_amount: u64,       // Tokens rewarded per rating
      pub total_ratings: u64,       // Total number of ratings submitted
      pub bump: u8,                 // PDA bump seed
  }
  ```

### 2. Rating Account (PDA)

- **Purpose**: Stores individual movie rating data
- **Seeds**: `["rating", movie_title, user_authority]`
- **Data Structure**:

  ```rust
  pub struct Rating {
      pub movie_title: String,      // Movie title (max 64 chars)
      pub rating: u8,               // Rating 1-10
      pub review: String,           // Optional review text (max 256 chars)
      pub owner: Pubkey,            // User who created the rating
      pub timestamp: i64,           // Unix timestamp of creation
      pub bump: u8,                 // PDA bump seed
  }
  ```

## Instructions

### 1. Initialize Admin (`init_admin`)

- **Purpose**: Creates the admin PDA and initializes the token mint
- **Accounts**:
  - `admin` (mut, PDA): Admin account to create
  - `authority` (signer): Admin wallet
  - `token_mint` (mut): Token mint account
  - `system_program`: System program
  - `token_program`: SPL Token program
- **Parameters**:
  - `reward_amount: u64`: Tokens to reward per rating

### 2. Initialize Rating (`init_rating`)

- **Purpose**: Creates a new movie rating PDA and rewards the user with tokens
- **Accounts**:
  - `rating` (mut, PDA): Rating account to create
  - `admin` (mut, PDA): Admin account
  - `user` (signer): User creating the rating
  - `user_token_account` (mut): User's token account to receive rewards
  - `token_mint` (mut): Token mint account
  - `system_program`: System program
  - `token_program`: SPL Token program
- **Parameters**:
  - `movie_title: String`: Movie title (max 64 chars)
  - `rating: u8`: Rating value (1-10)
  - `review: String`: Optional review text (max 256 chars)

### 3. Delete Rating (`delete_rating`)

- **Purpose**: Allows users to delete their own ratings
- **Accounts**:
  - `rating` (mut, PDA): Rating account to delete
  - `owner` (signer): Owner of the rating
  - `admin` (mut, PDA): Admin account (to update total count)
- **Parameters**:
  - `movie_title: String`: Movie title for PDA derivation

## Program Flow

1. **Admin Setup**:
   - Admin calls `init_admin` to create admin PDA and token mint
   - Sets reward amount for each rating submission

2. **User Rating Submission**:
   - User calls `init_rating` with movie title, rating, and optional review
   - Program creates rating PDA with user-specific seeds
   - Program mints reward tokens to user's token account
   - Admin account's total_ratings counter is incremented

3. **Rating Management**:
   - Users can delete their own ratings via `delete_rating`
   - Only the rating owner can delete their rating
   - Deleting a rating decrements the total count

## Security Considerations

- **PDA Seeds**: Use movie title + user authority to prevent duplicate ratings per user per movie
- **Authority Checks**: Only rating owners can delete their ratings
- **Admin Controls**: Only admin can initialize the program and control mint authority
- **Input Validation**:
  - Rating values must be 1-10
  - Movie titles max 64 characters
  - Reviews max 256 characters

## Token Economics

- Users receive a fixed reward amount for each rating submitted
- Admin controls the reward amount and can modify it
- Token mint is controlled by the admin PDA
- No tokens are burned when ratings are deleted (keeps incentive aligned)

## Future Enhancements

- Add rating aggregation/averaging functionality
- Implement reputation system based on rating history
- Add moderation capabilities for inappropriate content
- Enable rating updates (with potential token adjustment)
- Add movie metadata storage
- Implement rating verification/validation mechanisms
