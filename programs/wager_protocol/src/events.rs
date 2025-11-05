use anchor_lang::event;
use anchor_lang::prelude::*;
use anchor_lang::Discriminator;


#[event]
pub struct ProtocolInitialized {
    pub authority: Pubkey,
    pub protocol_fee_bps: u16,
    pub cancel_fee_bps: u16,
    pub dev_recipient: Pubkey,
}
#[event]
pub struct MarketCreated { pub market: Pubkey, pub creator: Pubkey, pub id: u64, pub question: String, pub end_time: i64 }
#[event]
pub struct BetPlaced { pub market: Pubkey, pub position: Pubkey, pub user: Pubkey, pub outcome: u8, pub amount: u64 }
#[event]
pub struct PositionIncreased { pub market: Pubkey, pub position: Pubkey, pub user: Pubkey, pub added_amount: u64 }
#[event]
pub struct Withdrawn { pub market: Pubkey, pub position: Pubkey, pub user: Pubkey, pub withdrawn: u64, pub payout: u64, pub fee: u64 }
#[event]
pub struct PositionCancelled { pub market: Pubkey, pub position: Pubkey, pub user: Pubkey, pub amount: u64, pub payout: u64, pub fee: u64 }
#[event]
pub struct MarketResolved { pub market: Pubkey, pub winner: u8 }
#[event]
pub struct WinningsClaimed { pub market: Pubkey, pub position: Pubkey, pub user: Pubkey, pub winnings: u64 }