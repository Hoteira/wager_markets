use anchor_lang::error_code;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid outcomes - must be exactly 2")]
    InvalidOutcomes,
    #[msg("Invalid end time - must be in future")]
    InvalidEndTime,
    #[msg("Invalid outcome selection")]
    InvalidOutcome,
    #[msg("Invalid amount - must be greater than 0")]
    InvalidAmount,
    #[msg("Market already resolved")]
    MarketResolved,
    #[msg("Market has ended")]
    MarketEnded,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Already resolved")]
    AlreadyResolved,
    #[msg("Market not ended yet")]
    MarketNotEnded,
    #[msg("Market not resolved yet")]
    MarketNotResolved,
    #[msg("Already claimed")]
    AlreadyClaimed,
    #[msg("Losing bet - cannot claim")]
    LosingBet,
}