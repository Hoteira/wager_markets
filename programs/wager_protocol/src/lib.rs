mod error;
mod constants;
mod structs;
mod events;

use structs::*;
use events::*;
use error::ErrorCode;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, TokenAccount, Token};

//Program ID to change !!
declare_id!("4HXdLHreKqwTNRDP4sVuUCzBEc6p89sXHp4auzzShbiB");

pub const PRECISION: u128 = 1_000_000_000_u128; // For fixed-point math

#[program]
pub mod wager_protocol {
    use std::str::FromStr;
    use super::*;

    pub fn initialize_protocol(
        ctx: Context<InitializeProtocol>,
        protocol_fee_bps: u16,
        cancel_fee_bps: u16,
        amm_fee: u16,
        authority_fee_recipient: Pubkey, // deployer's fee wallet
    ) -> Result<()> {

        let protocol = &mut ctx.accounts.protocol;
        protocol.authority = ctx.accounts.authority.key();
        protocol.authority_fee_recipient = authority_fee_recipient;
        protocol.protocol_fee_bps = protocol_fee_bps;
        protocol.cancel_fee_bps = cancel_fee_bps;
        protocol.amm_fee = amm_fee;
        protocol.market_count = 0;
        protocol.dev_recipient = Pubkey::from_str("8Nq7eMbvhZiPzZFeYutAoiHqF2uJTZZWwnBRzvkiUUid").unwrap();

        emit!(ProtocolInitialized {
        authority: protocol.authority,
        protocol_fee_bps,
        cancel_fee_bps,
        dev_recipient: protocol.dev_recipient,
    });
        Ok(())
    }

    /// Create a market. It requires exactly 2 outcomes.
    pub fn create_market(
        ctx: Context<CreateMarket>,
        question: String,
        outcomes: Vec<String>,
        end_time: i64,
    ) -> Result<()> {
        require!(outcomes.len() == 2, ErrorCode::InvalidOutcomes);
        require!(end_time > Clock::get()?.unix_timestamp, ErrorCode::InvalidEndTime);
        require_eq!(ctx.accounts.market_escrow.amount, 0, ErrorCode::EscrowNotEmpty);

        let market = &mut ctx.accounts.market;
        market.id = ctx.accounts.protocol.market_count;
        market.bump = ctx.bumps.market;
        market.creator = ctx.accounts.creator.key();
        market.question = question;
        market.outcomes = outcomes;
        market.end_time = end_time;
        market.resolved = false;
        market.total_volume = 0;
        market.outcome_pools = vec![0u64, 0u64];
        market.winning_outcome = None;
        market.position_count = 0;

        ctx.accounts.protocol.market_count = ctx.accounts.protocol.market_count
            .checked_add(1).ok_or(ErrorCode::AmountOverflow)?;

        emit!(MarketCreated {
            market: market.key(),
            creator: market.creator,
            id: market.id,
            question: market.question.clone(),
            end_time
        });
        Ok(())
    }

    /// Place a new bet -> creates a new Position PDA (history preserved).
    pub fn place_bet(
        ctx: Context<PlaceBet>,
        outcome: u8,
        amount: u64,
    ) -> Result<()> {
        require!(outcome < 2, ErrorCode::InvalidOutcome);
        require!(amount > 0, ErrorCode::InvalidAmount);

        let market = &mut ctx.accounts.market;
        require!(!market.resolved, ErrorCode::MarketResolved);
        require!(Clock::get()?.unix_timestamp < market.end_time, ErrorCode::MarketEnded);

        // Transfer tokens
        let cpi_accounts = token::Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.market_escrow.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        token::transfer(CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts), amount)?;

        // Add to pool
        market.outcome_pools[outcome as usize] = market.outcome_pools[outcome as usize]
            .checked_add(amount).ok_or(ErrorCode::AmountOverflow)?;
        market.total_volume = market.total_volume.checked_add(amount).ok_or(ErrorCode::AmountOverflow)?;

        // Create position tracking
        let position = &mut ctx.accounts.position;
        position.id = market.position_count;
        position.bump = ctx.bumps.position;
        position.user = ctx.accounts.user.key();
        position.market = market.key();
        position.outcome = outcome;
        position.amount = amount;
        position.claimed = false;
        position.ts = Clock::get()?.unix_timestamp;
        market.position_count = market.position_count.checked_add(1).ok_or(ErrorCode::AmountOverflow)?;

        emit!(BetPlaced {
            market: market.key(),
            position: position.key(),
            user: position.user,
            outcome,
            amount
        });

        Ok(())
    }

    /// Increase funds in an existing position (convenience). Snapshots the last added chunk and updates ts.
    pub fn increase_position(
        ctx: Context<IncreasePosition>,
        added_amount: u64,
    ) -> Result<()> {
        require!(added_amount > 0, ErrorCode::InvalidAmount);
        let market = &mut ctx.accounts.market;
        require!(!market.resolved, ErrorCode::MarketResolved);
        require!(Clock::get()?.unix_timestamp < market.end_time, ErrorCode::MarketEnded);

        let position = &mut ctx.accounts.position;
        require!(position.user == ctx.accounts.user.key(), ErrorCode::PositionOwnerMismatch);

        // Transfer tokens
        let cpi_accounts = token::Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.market_escrow.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        token::transfer(CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts), added_amount)?;

        // Update pools
        let outcome_index = position.outcome as usize;
        market.outcome_pools[outcome_index] = market.outcome_pools[outcome_index]
            .checked_add(added_amount).ok_or(ErrorCode::AmountOverflow)?;
        market.total_volume = market.total_volume.checked_add(added_amount).ok_or(ErrorCode::AmountOverflow)?;

        // Update position
        position.amount = position.amount.checked_add(added_amount).ok_or(ErrorCode::AmountOverflow)?;
        position.ts = Clock::get()?.unix_timestamp;

        emit!(PositionIncreased {
            market: market.key(),
            position: position.key(),
            user: position.user,
            added_amount
        });
        Ok(())
    }

    /// Withdraw the partial amount from a Position before the market end (AMM-style sell).
    pub fn withdraw_from_position(
        ctx: Context<WithdrawFromPosition>,
        amount_to_withdraw: u64,
        min_payout: u64,
    ) -> Result<()> {
        require!(amount_to_withdraw > 0, ErrorCode::InvalidAmount);

        let market = &mut ctx.accounts.market;
        let protocol = &ctx.accounts.protocol;
        let position = &mut ctx.accounts.position;

        require_keys_eq!(
            ctx.accounts.authority_fee_recipient.key(),
            protocol.authority_fee_recipient,
            ErrorCode::InvalidFeeRecipient
        );

        require!(Clock::get()?.unix_timestamp < market.end_time, ErrorCode::MarketAlreadyEndedForModification);
        require!(position.user == ctx.accounts.user.key(), ErrorCode::PositionOwnerMismatch);
        require!(amount_to_withdraw <= position.amount, ErrorCode::WithdrawAmountExceedsPosition);

        let idx = position.outcome as usize;
        let pool_outcome = market.outcome_pools[idx] as u128;
        let pool_other = market.outcome_pools[1 - idx] as u128;

        require!(pool_outcome > amount_to_withdraw as u128, ErrorCode::InsufficientLiquidity);
        require!(pool_other > 0, ErrorCode::InsufficientLiquidity);

        // Constant product: k = x * y
        let k = pool_outcome.checked_mul(pool_other).ok_or(ErrorCode::AmountOverflow)?;

        // Remove tokens from the outcome pool
        let new_pool_outcome = pool_outcome.checked_sub(amount_to_withdraw as u128)
            .ok_or(ErrorCode::AmountOverflow)?;

        // Calculate the new pool to maintain k
        let new_pool_other = k.checked_div(new_pool_outcome).ok_or(ErrorCode::AmountOverflow)?;

        let payout_gross = new_pool_other.checked_sub(pool_other).ok_or(ErrorCode::AmountOverflow)?;
        require!(payout_gross <= u64::MAX as u128, ErrorCode::AmountOverflow);
        let payout_gross_u64 = payout_gross as u64;

        // Apply fees
        let amm_fee = payout_gross
            .checked_mul(ctx.accounts.protocol.amm_fee as u128).ok_or(ErrorCode::AmountOverflow)?
            .checked_div(10_000).ok_or(ErrorCode::AmountOverflow)?;

        let cancel_fee = payout_gross
            .checked_mul(protocol.cancel_fee_bps as u128).ok_or(ErrorCode::AmountOverflow)?
            .checked_div(10_000).ok_or(ErrorCode::AmountOverflow)?;

        let total_fee = amm_fee.checked_add(cancel_fee).ok_or(ErrorCode::AmountOverflow)?;
        require!(total_fee <= u64::MAX as u128, ErrorCode::AmountOverflow);
        let total_fee_u64 = total_fee as u64;

        let payout_net = payout_gross_u64.checked_sub(total_fee_u64).ok_or(ErrorCode::AmountOverflow)?;

        // Slippage protection
        require!(payout_net >= min_payout, ErrorCode::SlippageExceeded);

        // Update pools to maintain AMM invariant
        position.amount = position.amount.checked_sub(amount_to_withdraw).ok_or(ErrorCode::AmountOverflow)?;
        market.outcome_pools[idx] = new_pool_outcome.try_into().map_err(|_| ErrorCode::AmountOverflow)?;
        market.outcome_pools[1 - idx] = new_pool_other.try_into().map_err(|_| ErrorCode::AmountOverflow)?;
        market.total_volume = market.total_volume.checked_sub(amount_to_withdraw).ok_or(ErrorCode::AmountOverflow)?;

        // Transfers
        let id_bytes = market.id.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[b"market", id_bytes.as_ref(), &[market.bump]]];
        let cpi_program = ctx.accounts.token_program.to_account_info();

        // Payout to the user
        token::transfer(
            CpiContext::new_with_signer(
                cpi_program.clone(),
                token::Transfer {
                    from: ctx.accounts.market_escrow.to_account_info(),
                    to: ctx.accounts.user_token_account.to_account_info(),
                    authority: market.to_account_info(),
                },
                signer_seeds
            ),
            payout_net
        )?;

        // Fees: need to add tokens back to the opposite pool
        let tokens_to_add = payout_gross_u64.checked_sub(payout_net).ok_or(ErrorCode::AmountOverflow)?;
        if tokens_to_add > 0 {
            market.outcome_pools[1 - idx] = market.outcome_pools[1 - idx]
                .checked_add(tokens_to_add).ok_or(ErrorCode::AmountOverflow)?;
        }

        // Distribute protocol fees
        distribute_fees(
            &ctx.accounts.market_escrow,
            &ctx.accounts.authority_fee_recipient,
            &ctx.accounts.dev_token_account,
            &market.to_account_info(),
            &ctx.accounts.token_program,
            signer_seeds,
            total_fee_u64
        )?;

        emit!(Withdrawn {
            market: market.key(),
            position: position.key(),
            user: position.user,
            withdrawn: amount_to_withdraw,
            payout: payout_net,
            fee: total_fee_u64
        });

        Ok(())
    }

    /// Cancel the entire position BEFORE market end (AMM-style full sell with cancel fee).
    pub fn cancel_position(
        ctx: Context<CancelPosition>,
        min_payout: u64,
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let position = &mut ctx.accounts.position;
        let protocol = &ctx.accounts.protocol;

        require_keys_eq!(
            ctx.accounts.authority_fee_recipient.key(),
            protocol.authority_fee_recipient,
            ErrorCode::InvalidFeeRecipient
        );
        require!(Clock::get()?.unix_timestamp < market.end_time, ErrorCode::MarketAlreadyEndedForModification);
        require!(position.user == ctx.accounts.user.key(), ErrorCode::PositionOwnerMismatch);
        require!(!position.claimed, ErrorCode::AlreadyClaimed);
        require!(position.amount > 0, ErrorCode::InvalidAmount);

        let amount_to_return = position.amount;
        let idx = position.outcome as usize;
        let pool_outcome = market.outcome_pools[idx] as u128;
        let pool_other = market.outcome_pools[1 - idx] as u128;

        require!(pool_outcome > amount_to_return as u128, ErrorCode::InsufficientLiquidity);
        require!(pool_other > 0, ErrorCode::InsufficientLiquidity);

        // Constant product AMM
        let k = pool_outcome.checked_mul(pool_other).ok_or(ErrorCode::AmountOverflow)?;
        let new_pool_outcome = pool_outcome.checked_sub(amount_to_return as u128)
            .ok_or(ErrorCode::AmountOverflow)?;
        let new_pool_other = k.checked_div(new_pool_outcome).ok_or(ErrorCode::AmountOverflow)?;
        let payout_gross = new_pool_other.checked_sub(pool_other).ok_or(ErrorCode::AmountOverflow)?;

        require!(payout_gross <= u64::MAX as u128, ErrorCode::AmountOverflow);
        let payout_gross_u64 = payout_gross as u64;

        // Fees
        let amm_fee = payout_gross
            .checked_mul(ctx.accounts.protocol.amm_fee as u128).ok_or(ErrorCode::AmountOverflow)?
            .checked_div(10_000u128).ok_or(ErrorCode::AmountOverflow)?;

        let cancel_fee = payout_gross
            .checked_mul(protocol.cancel_fee_bps as u128).ok_or(ErrorCode::AmountOverflow)?
            .checked_div(10_000u128).ok_or(ErrorCode::AmountOverflow)?;

        let total_fee = amm_fee.checked_add(cancel_fee).ok_or(ErrorCode::AmountOverflow)?;
        require!(total_fee <= u64::MAX as u128, ErrorCode::AmountOverflow);
        let total_fee_u64 = total_fee as u64;

        let payout_net = payout_gross_u64.checked_sub(total_fee_u64).ok_or(ErrorCode::AmountOverflow)?;
        require!(payout_net >= min_payout, ErrorCode::SlippageExceeded);

        // Update pools
        market.outcome_pools[idx] = new_pool_outcome.try_into().map_err(|_| ErrorCode::AmountOverflow)?;
        market.outcome_pools[1 - idx] = new_pool_other.try_into().map_err(|_| ErrorCode::AmountOverflow)?;
        market.total_volume = market.total_volume.checked_sub(amount_to_return).ok_or(ErrorCode::AmountOverflow)?;

        // Transfers
        let id_bytes = market.id.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[b"market", id_bytes.as_ref(), &[market.bump]]];
        let cpi_program = ctx.accounts.token_program.to_account_info();

        token::transfer(
            CpiContext::new_with_signer(
                cpi_program.clone(),
                token::Transfer {
                    from: ctx.accounts.market_escrow.to_account_info(),
                    to: ctx.accounts.user_token_account.to_account_info(),
                    authority: market.to_account_info(),
                },
                signer_seeds
            ),
            payout_net
        )?;

        // Add fees back to the opposite pool
        let tokens_to_add = payout_gross_u64.checked_sub(payout_net).ok_or(ErrorCode::AmountOverflow)?;
        if tokens_to_add > 0 {
            market.outcome_pools[1 - idx] = market.outcome_pools[1 - idx]
                .checked_add(tokens_to_add).ok_or(ErrorCode::AmountOverflow)?;
        }

        distribute_fees(
            &ctx.accounts.market_escrow,
            &ctx.accounts.authority_fee_recipient,
            &ctx.accounts.dev_token_account,
            &market.to_account_info(),
            &ctx.accounts.token_program,
            signer_seeds,
            total_fee_u64
        )?;

        position.amount = 0;
        position.claimed = true;

        emit!(PositionCancelled {
            market: market.key(),
            position: position.key(),
            user: position.user,
            amount: amount_to_return,
            payout: payout_net,
            fee: total_fee_u64
        });

        Ok(())
    }


    /// Resolve market (unchanged behavior — but note: creator resolving is trustful)
    pub fn resolve_market(
        ctx: Context<ResolveMarket>,
        winning_outcome: u8,
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;
        require!(ctx.accounts.creator.key() == market.creator, ErrorCode::Unauthorized);
        require!(!market.resolved, ErrorCode::AlreadyResolved);
        require!(Clock::get()?.unix_timestamp >= market.end_time, ErrorCode::MarketNotEnded);
        require!(winning_outcome < 2, ErrorCode::InvalidOutcome);

        market.resolved = true;
        market.winning_outcome = Some(winning_outcome);

        emit!(MarketResolved {
            market: market.key(),
            winner: winning_outcome
        });
        Ok(())
    }

    /// Claim winnings after the end of the market
    pub fn claim_winnings(ctx: Context<ClaimWinnings>) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let position = &mut ctx.accounts.position;
        let protocol = &ctx.accounts.protocol;

        require!(market.resolved, ErrorCode::MarketNotResolved);
        require!(position.user == ctx.accounts.user.key(), ErrorCode::PositionOwnerMismatch);
        require!(!position.claimed, ErrorCode::AlreadyClaimed);
        require!(position.amount > 0, ErrorCode::InvalidAmount);

        // Check if user bet on winning outcome
        let winning_outcome = market.winning_outcome.ok_or(ErrorCode::MarketNotResolved)?;
        require!(position.outcome == winning_outcome, ErrorCode::InvalidOutcome);

        let winner_pool = market.outcome_pools[winning_outcome as usize];
        let loser_pool = market.outcome_pools[1 - winning_outcome as usize];

        require!(winner_pool > 0, ErrorCode::NoWinnersRemaining);

        // User's share of losing pool = (position_amount / winner_pool) * loser_pool
        let user_share = (position.amount as u128)
            .checked_mul(loser_pool as u128).ok_or(ErrorCode::AmountOverflow)?
            .checked_div(winner_pool as u128).ok_or(ErrorCode::AmountOverflow)?;

        // Total payout = original stake + winnings
        let gross_payout = (position.amount as u128)
            .checked_add(user_share).ok_or(ErrorCode::AmountOverflow)?;

        require!(gross_payout <= u64::MAX as u128, ErrorCode::AmountOverflow);
        let gross_payout_u64 = gross_payout as u64;

        // Apply protocol fee
        let protocol_fee = gross_payout
            .checked_mul(protocol.protocol_fee_bps as u128).ok_or(ErrorCode::AmountOverflow)?
            .checked_div(10_000).ok_or(ErrorCode::AmountOverflow)?;

        require!(protocol_fee <= u64::MAX as u128, ErrorCode::AmountOverflow);
        let protocol_fee_u64 = protocol_fee as u64;

        let net_payout = gross_payout_u64.checked_sub(protocol_fee_u64)
            .ok_or(ErrorCode::AmountOverflow)?;

        // Transfer winnings
        let id_bytes = market.id.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[b"market", id_bytes.as_ref(), &[market.bump]]];

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.market_escrow.to_account_info(),
                    to: ctx.accounts.user_token_account.to_account_info(),
                    authority: market.to_account_info(),
                },
                signer_seeds
            ),
            net_payout
        )?;

        // Distribute fees
        distribute_fees(
            &ctx.accounts.market_escrow,
            &ctx.accounts.authority_fee_recipient,
            &ctx.accounts.dev_token_account,
            &market.to_account_info(),
            &ctx.accounts.token_program,
            signer_seeds,
            protocol_fee_u64
        )?;

        position.claimed = true;

        emit!(WinningsClaimed {
        market: market.key(),
        position: position.key(),
        user: position.user,
        winnings: net_payout
    });

        Ok(())
    }
}

fn distribute_fees<'info>(
    escrow: &Account<'info, TokenAccount>,
    authority_account: &AccountInfo<'info>,
    dev_account: &Account<'info, TokenAccount>,
    authority: &AccountInfo<'info>,
    token_program: &Program<'info, Token>,
    signer_seeds: &[&[&[u8]]],
    total_fee: u64,
) -> Result<()> {
    if total_fee == 0 {
        return Ok(());
    }

    let half = total_fee / 2;
    let remainder = total_fee - half;

    token::transfer(
        CpiContext::new_with_signer(
            token_program.to_account_info(),
            token::Transfer {
                from: escrow.to_account_info(),
                to: authority_account.to_account_info(),
                authority: authority.clone(),
            },
            signer_seeds
        ),
        half
    )?;

    token::transfer(
        CpiContext::new_with_signer(
            token_program.to_account_info(),
            token::Transfer {
                from: escrow.to_account_info(),
                to: dev_account.to_account_info(),
                authority: authority.clone(),
            },
            signer_seeds
        ),
        remainder
    )?;

    Ok(())
}