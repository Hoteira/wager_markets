use anchor_lang::error_code;
#[error_code]
pub enum ErrorCode {
    #[msg("Invalid outcomes")]
    InvalidOutcomes,
    #[msg("Invalid end time")]
    InvalidEndTime,
    #[msg("Escrow not empty")]
    EscrowNotEmpty,
    #[msg("Invalid outcome")]
    InvalidOutcome,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Market resolved")]
    MarketResolved,
    #[msg("Market ended")]
    MarketEnded,
    #[msg("Amount overflow")]
    AmountOverflow,
    #[msg("Market already ended")]
    MarketAlreadyEndedForModification,
    #[msg("Position owner mismatch")]
    PositionOwnerMismatch,
    #[msg("Withdraw amount exceeds position")]
    WithdrawAmountExceedsPosition,
    #[msg("Insufficient escrow")]
    InsufficientEscrow,
    #[msg("Already claimed")]
    AlreadyClaimed,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Already resolved")]
    AlreadyResolved,
    #[msg("Market not ended")]
    MarketNotEnded,
    #[msg("Market not resolved")]
    MarketNotResolved,
    #[msg("Insufficient liquidity")]
    InsufficientLiquidity,
    #[msg("Slippage exceeded")]
    SlippageExceeded,
    #[msg("No winners remaining")]
    NoWinnersRemaining,
}