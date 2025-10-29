use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

#[account]
#[derive(InitSpace)]
pub struct Protocol {
    pub authority: Pubkey,        // 32
    pub protocol_fee_bps: u16,    // 2
    pub market_count: u64,        // 8
}

#[account]
#[derive(InitSpace)]
pub struct Market {
    pub creator: Pubkey,          // 32
    #[max_len(200)]
    pub question: String,         // 4 + 200
    #[max_len(2, 50)]
    pub outcomes: Vec<String>,    // 4 + (2 * (4 + 50))
    pub end_time: i64,            // 8
    pub resolved: bool,           // 1
    pub winning_outcome: Option<u8>, // 1 + 1
    pub total_volume: u64,        // 8
    #[max_len(2)]
    pub outcome_pools: Vec<u64>,  // 4 + (2 * 8)
}

#[account]
#[derive(InitSpace)]
pub struct Position {
    pub user: Pubkey,             // 32
    pub market: Pubkey,           // 32
    pub outcome: u8,              // 1
    pub amount: u64,              // 8
    pub claimed: bool,            // 1
}

// Context Structs
#[derive(Accounts)]
pub struct InitializeProtocol<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Protocol::INIT_SPACE,
        seeds = [b"protocol"],
        bump
    )]
    pub protocol: Account<'info, Protocol>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateMarket<'info> {
    #[account(
        init,
        payer = creator,
        space = 8 + Market::INIT_SPACE,
        seeds = [b"market", protocol.market_count.to_le_bytes().as_ref()],
        bump
    )]
    pub market: Account<'info, Market>,
    #[account(mut)]
    pub protocol: Account<'info, Protocol>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PlaceBet<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,
    #[account(
        init,
        payer = user,
        space = 8 + Position::INIT_SPACE
    )]
    pub position: Account<'info, Position>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub market_escrow: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ResolveMarket<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,
    pub creator: Signer<'info>,
}

#[derive(Accounts)]
pub struct ClaimWinnings<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,
    #[account(mut)]
    pub position: Account<'info, Position>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub market_escrow: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}