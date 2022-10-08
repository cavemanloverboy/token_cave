use anchor_lang::prelude::*;

#[error_code]
pub enum TokenCaveError {

    #[msg("Depositor did not request unlock")]
    DidNotRequestUnlock,

    #[msg("You are not the depositor")]
    Unauthorized,

    #[msg("The lock duration has not yet elapsed")]
    LockIsActive,

    #[msg("There is already an unlock happening")]
    UnlockAlreadyActive,

    #[msg("The maximum allowed lock duration is MAX_TIMELOCK_DURATION. Refer to IDL")]
    DurationExceedsMaximum,

    #[msg("You supplied a token account that does not belong to the backup address")]
    IncorrectBackupTokenAccount,

}