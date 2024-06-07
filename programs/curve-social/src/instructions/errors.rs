use anchor_lang::error_code;


#[error_code]
pub enum CurveSocialError {
    #[msg("Global Already Initialized")]
    AlreadyInitialized,
    #[msg("Global Not Initialized")]
    NotInitialized,
    #[msg("Invalid Authority")]
    InvalidAuthority,
    #[msg("Bonding Curve Complete")]
    BondingCurveComplete,
    #[msg("Insufficient Tokens")]
    InsufficientTokens,
    #[msg("Insufficient SOL")]
    InsufficientSOL
}