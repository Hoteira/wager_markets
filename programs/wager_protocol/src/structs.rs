use anchor_lang::{account, Accounts};
use anchor_lang::prelude::{Account, Program, Pubkey, Rent, Signer, System, Sysvar};
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};
use anchor_lang::prelude::*;
use anchor_lang::Discriminator;


#[account]
pub struct Market {
    pub id: u64,                // 8
    pub bump: u8,               // 1
    pub creator: Pubkey,        // 32
    pub question: String,       // 4 + up to N
    pub outcomes: Vec<String>,  // 4 + each string
    pub end_time: i64,          // 8
    pub resolved: bool,         // 1
    pub winning_outcome: Option<u8>, // 1 + 1
    pub total_volume: u64,      // 8
    pub outcome_pools: Vec<u64>,// 4 + (2*8)
    pub position_count: u64,    // 8
}

impl Market {
    pub const INIT_SPACE: usize = 8 + 1 + 32 + (4 + 200) + (4 + 2 * (4 + 50)) + 8 + 1 + 2 + 8 + (4 + 16) + 8;
}

#[account]
pub struct Protocol {
    pub authority: Pubkey,        // 32
    pub protocol_fee_bps: u16,    // 2 (fee taken on winnings)
    pub cancel_fee_bps: u16,      // 2 (fee when cancelling/withdrawing before end)
    pub market_count: u64,        // 8
    pub dev_recipient: Pubkey,    // 32
}

impl Protocol {
    pub const INIT_SPACE: usize = 32 + 2 + 2 + 8 + 32;
}

#[account]
pub struct Position {
    pub id: u64,                      // 8  (unique per market)
    pub bump: u8,                     // 1
    pub user: Pubkey,                 // 32
    pub market: Pubkey,               // 32
    pub outcome: u8,                  // 1
    pub amount: u64,                  // 8
    pub claimed: bool,                // 1
    pub ts: i64,                      // 8 (timestamp when bet placed or last increased)
}
impl Position {
    pub const INIT_SPACE: usize = 8 + 1 + 32 + 32 + 1 + 8 + 1 + 8 + 8 + 8;
}

#[derive(Accounts)]
pub struct InitializeProtocol<'info> {
    #[account(init, payer = authority, space = 8 + Protocol::INIT_SPACE, seeds = [b"protocol"], bump)]
    pub protocol: Account<'info, Protocol>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateMarket<'info> {
    #[account(init, payer = creator, space = 8 + Market::INIT_SPACE, seeds = [b"market", protocol.market_count.to_le_bytes().as_ref()], bump)]
    pub market: Account<'info, Market>,
    #[account(mut)]
    pub protocol: Account<'info, Protocol>,
    #[account(mut)]
    pub creator: Signer<'info>,

    // market escrow ATA (market PDA is owner)
    #[account(
        init_if_needed,
        payer = creator,
        associated_token::mint = token_mint,
        associated_token::authority = market
    )]
    pub market_escrow: Account<'info, TokenAccount>,

    pub token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct PlaceBet<'info> {
    #[account(mut, seeds = [b"market", market.id.to_le_bytes().as_ref()], bump = market.bump)]
    pub market: Account<'info, Market>,

    // New position PDA uses market.position_count as unique index
    #[account(
        init,
        payer = user,
        space = 8 + Position::INIT_SPACE,
        seeds = [b"position", user.key().as_ref(), market.key().as_ref(), market.position_count.to_le_bytes().as_ref()],
        bump
    )]
    pub position: Account<'info, Position>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, token::mint = token_mint, token::authority = user)]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(mut, associated_token::mint = token_mint, associated_token::authority = market)]
    pub market_escrow: Account<'info, TokenAccount>,

    pub token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct IncreasePosition<'info> {
    #[account(mut, seeds = [b"market", market.id.to_le_bytes().as_ref()], bump = market.bump)]
    pub market: Account<'info, Market>,

    #[account(mut, seeds = [b"position", position.user.key().as_ref(), market.key().as_ref(), position.id.to_le_bytes().as_ref()], bump = position.bump)]
    pub position: Account<'info, Position>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, token::mint = token_mint, token::authority = user)]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(mut, associated_token::mint = token_mint, associated_token::authority = market)]
    pub market_escrow: Account<'info, TokenAccount>,

    pub token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct WithdrawFromPosition<'info> {
    #[account(mut, seeds = [b"market", market.id.to_le_bytes().as_ref()], bump = market.bump)]
    pub market: Account<'info, Market>,

    #[account(mut, seeds = [b"position", position.user.key().as_ref(), market.key().as_ref(), position.id.to_le_bytes().as_ref()], bump = position.bump)]
    pub position: Account<'info, Position>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, token::mint = token_mint, token::authority = user)]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(mut, associated_token::mint = token_mint, associated_token::authority = market)]
    pub market_escrow: Account<'info, TokenAccount>,

    // protocol & dev token accounts (must exist)
    #[account(seeds = [b"protocol"], bump)]
    pub protocol: Account<'info, Protocol>,

    #[account(mut, associated_token::mint = token_mint, associated_token::authority = protocol.authority)]
    pub protocol_token_account: Account<'info, TokenAccount>,

    #[account(mut, associated_token::mint = token_mint, associated_token::authority = protocol.dev_recipient)]
    pub dev_token_account: Account<'info, TokenAccount>,

    pub token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CancelPosition<'info> {
    #[account(mut, seeds = [b"market", market.id.to_le_bytes().as_ref()], bump = market.bump)]
    pub market: Account<'info, Market>,

    #[account(mut, seeds = [b"position", position.user.key().as_ref(), market.key().as_ref(), position.id.to_le_bytes().as_ref()], bump = position.bump)]
    pub position: Account<'info, Position>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, token::mint = token_mint, token::authority = user)]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(mut, associated_token::mint = token_mint, associated_token::authority = market)]
    pub market_escrow: Account<'info, TokenAccount>,

    #[account(seeds = [b"protocol"], bump)]
    pub protocol: Account<'info, Protocol>,

    #[account(mut, associated_token::mint = token_mint, associated_token::authority = protocol.authority)]
    pub protocol_token_account: Account<'info, TokenAccount>,

    #[account(mut, associated_token::mint = token_mint, associated_token::authority = protocol.dev_recipient)]
    pub dev_token_account: Account<'info, TokenAccount>,

    pub token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ResolveMarket<'info> {
    #[account(mut, seeds = [b"market", market.id.to_le_bytes().as_ref()], bump = market.bump)]
    pub market: Account<'info, Market>,
    pub creator: Signer<'info>,
}

#[derive(Accounts)]
pub struct ClaimWinnings<'info> {
    #[account(mut, seeds = [b"market", market.id.to_le_bytes().as_ref()], bump = market.bump)]
    pub market: Account<'info, Market>,

    #[account(seeds = [b"protocol"], bump)]
    pub protocol: Account<'info, Protocol>,

    #[account(mut, seeds = [b"position", position.user.key().as_ref(), market.key().as_ref(), position.id.to_le_bytes().as_ref()], bump = position.bump)]
    pub position: Account<'info, Position>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, token::mint = token_mint, token::authority = user)]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(mut, associated_token::mint = token_mint, associated_token::authority = market)]
    pub market_escrow: Account<'info, TokenAccount>,

    #[account(mut, associated_token::mint = token_mint, associated_token::authority = protocol.authority)]
    pub protocol_token_account: Account<'info, TokenAccount>,

    #[account(mut, associated_token::mint = token_mint, associated_token::authority = protocol.dev_recipient)]
    pub dev_token_account: Account<'info, TokenAccount>,

    pub token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}
