pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
pub mod structs;

use error::ErrorCode;
use structs::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer};

declare_id!("4jHsbdQ3VvtrxzGC1Mx7bNYGrdKge2imTDAqJC2QHYsB");

#[program]
pub mod wager_protocol {
    use super::*;

    pub fn initialize_protocol(ctx: Context<InitializeProtocol>) -> Result<()> {
        let protocol = &mut ctx.accounts.protocol;
        protocol.authority = ctx.accounts.authority.key();
        protocol.protocol_fee_bps = 50; // 0.5%
        protocol.market_count = 0;
        
        msg!("WAGER Protocol initialized!");
        Ok(())
    }

    pub fn create_market(
        ctx: Context<CreateMarket>,
        question: String,
        outcomes: Vec<String>,
        end_time: i64,
    ) -> Result<()> {
        require!(outcomes.len() == 2, ErrorCode::InvalidOutcomes);
        require!(end_time > Clock::get()?.unix_timestamp, ErrorCode::InvalidEndTime);

        let market = &mut ctx.accounts.market;
        market.creator = ctx.accounts.creator.key();
        market.question = question;
        market.outcomes = outcomes;
        market.end_time = end_time;
        market.resolved = false;
        market.total_volume = 0;
        market.outcome_pools = vec![0, 0];
        market.winning_outcome = None;

        msg!("Market created: {}", market.question);
        Ok(())
    }

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

        // Transfer [USDC] from user to market escrow
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.market_escrow.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        // Update market state
        market.outcome_pools[outcome as usize] += amount;
        market.total_volume += amount;

        // Create position account
        let position = &mut ctx.accounts.position;
        position.user = ctx.accounts.user.key();
        position.market = market.key();
        position.outcome = outcome;
        position.amount = amount;
        position.claimed = false;

        msg!("Bet placed: {} on outcome {}", amount, outcome);
        Ok(())
    }

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

        msg!("Market resolved. Winning outcome: {}", winning_outcome);
        Ok(())
    }

    pub fn claim_winnings(ctx: Context<ClaimWinnings>) -> Result<()> {
        let market = &ctx.accounts.market;
        let position = &mut ctx.accounts.position;

        require!(market.resolved, ErrorCode::MarketNotResolved);
        require!(!position.claimed, ErrorCode::AlreadyClaimed);
        require!(position.outcome == market.winning_outcome.unwrap(), ErrorCode::LosingBet);

        // Calculate winnings
        let winning_pool = market.outcome_pools[position.outcome as usize];
        let losing_pool = market.outcome_pools[1 - position.outcome as usize];

        let share_ratio = (position.amount as f64) / (winning_pool as f64);
        let winnings = position.amount + ((losing_pool as f64) * share_ratio) as u64;

        position.claimed = true;

        msg!("Winnings claimed: {}", winnings);
        Ok(())
    }
}