use anchor_lang::{
    account,
    prelude::{borsh, Clock, Pubkey, SolanaSysvar},
    solana_program::clock::UnixTimestamp,
    AnchorDeserialize, AnchorSerialize, Discriminator, InitSpace,
};

/// The lock for the SVM proxy.
#[account]
#[derive(InitSpace)]
pub struct Lock {
    /// The timestamp at which the lock expires.
    locked_until: i64,
}

impl Lock {
    /// Creates a new lock with the given expiration timestamp.
    pub fn new(locked_until: UnixTimestamp) -> Self {
        Self { locked_until }
    }

    /// Checks if the lock is expired.
    pub fn is_expired(&self) -> bool {
        Clock::get()
            .map(|clock| clock.unix_timestamp > self.locked_until)
            .unwrap_or(false)
    }

    /// Refreshes the lock with a new expiration timestamp.
    pub fn refresh(&mut self, new_locked_until: UnixTimestamp) {
        self.locked_until = new_locked_until;
    }
}

/// Configuration for the lock.
#[account]
#[derive(InitSpace)]
pub struct LockConfig {
    /// The length of time for the lock to last in seconds.
    pub lock_duration: u16,

    /// The caller allowed to refresh the lock.
    caller: Pubkey,
}

impl LockConfig {
    /// Creates a new lock configuration with the given lock time.
    pub fn new(lock_duration: u16, caller: Pubkey) -> Self {
        Self {
            lock_duration,
            caller,
        }
    }

    /// Returns the lock duration in seconds.
    pub fn lock_duration(&self) -> i64 {
        self.lock_duration as i64
    }

    /// Checks if the caller is authorized to refresh the lock.
    pub fn is_authorized(&self, caller: Pubkey) -> bool {
        self.caller == caller
    }
}
